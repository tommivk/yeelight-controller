mod bulb;
use bulb::Bulb;
use gtk::{prelude::*, Adjustment, Label};
use gtk::{
    Application, ApplicationWindow, Box, Button, ColorChooserWidgetBuilder, Orientation, Scale,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::{self};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};

const MULTI_CAST_ADRESS: &str = "239.255.255.250:1982";

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind("0.0.0.0:3480").await.unwrap();

    let bulbs: Vec<Bulb> = get_bulbs(socket).await;

    start_app(bulbs);
}

fn start_app(bulbs: Vec<Bulb>) {
    let application = Application::builder()
        .application_id("tommivk.yeelightController")
        .build();

    application.connect_activate(move |app| {
        let bulbs = Arc::new(RwLock::new(bulbs.to_owned()));
        let active_bulb = Arc::new(RwLock::new(bulbs.read().unwrap()[0].clone()));
        let bulb_id_buttons = Arc::new(RwLock::new(Vec::<Button>::new()));
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Yeelight Controller")
            .default_width(350)
            .default_height(350)
            .build();

        let button_row = Box::new(Orientation::Vertical, 3);

        for bulb in bulbs.read().unwrap().to_owned().into_iter() {
            let active = &active_bulb.read().unwrap().location.to_owned();

            let label = if &bulb.location == active {
                format!("{}{}", &bulb.id, " (Selected)")
            } else {
                bulb.id.to_string()
            };

            let id_button = Button::with_label(&label);

            bulb_id_buttons.write().unwrap().push(id_button.clone());

            id_button.connect_clicked({
                println!("{:?}", &bulb);
                let active_bulb = Arc::clone(&active_bulb);
                let id_button = id_button.clone();
                let bulb_id_buttons = Arc::clone(&bulb_id_buttons);
                move |_| {
                    *active_bulb.write().unwrap() = bulb.clone();

                    for bulb in bulb_id_buttons.read().unwrap().to_owned().into_iter() {
                        let label = bulb.label().unwrap().to_string().replace(" (Selected)", "");
                        Button::set_label(&bulb, &label);
                    }

                    let label = format!("{}{}", &bulb.id, " (Selected)");
                    Button::set_label(&id_button, &label);
                }
            });
            button_row.pack_start(&id_button, false, false, 2);
        }

        window.add(&button_row);

        let off_button = Button::with_label("Off");

        off_button.connect_clicked({
            let active_bulb = Arc::clone(&active_bulb);
            move |_| {
                send_command(
                    &active_bulb.read().unwrap().location,
                    "set_power",
                    "\"off\"",
                )
            }
        });

        let on_button = Button::with_label("On");

        on_button.connect_clicked({
            let active_bulb = Arc::clone(&active_bulb);
            move |_| send_command(&active_bulb.read().unwrap().location, "set_power", "\"on\"")
        });

        button_row.pack_start(&on_button, true, true, 2);
        button_row.pack_start(&off_button, true, true, 2);

        let color_picker = ColorChooserWidgetBuilder::new()
            .show_editor(true)
            .use_alpha(false)
            .margin_bottom(20)
            .margin_top(20)
            .build();

        color_picker.connect_rgba_notify({
            let active_bulb = Arc::clone(&active_bulb);
            move |picker| {
                let rgba = picker.rgba();

                let red = (255.0 * rgba.red).round() as i32;
                let green = (255.0 * rgba.green).round() as i32;
                let blue = (255.0 * rgba.blue).round() as i32;

                let color_value = (red * 65536) + (green * 256) + blue;

                send_command(
                    &active_bulb.read().unwrap().location,
                    "set_rgb",
                    &format!("{}", color_value),
                )
            }
        });

        button_row.pack_start(&color_picker, true, true, 3);

        let brightness_slider = Scale::new(
            Orientation::Horizontal,
            Some(&Adjustment::new(100.0, 1.0, 100.0, 1.0, 0.0, 0.0)),
        );

        brightness_slider.connect_change_value({
            let active_bulb = Arc::clone(&active_bulb);
            move |_s, _t, value| {
                let mut value = value.round() as i32;

                if value > 100 {
                    value = 100
                }

                send_command(
                    &active_bulb.read().unwrap().location,
                    "set_bright",
                    &format!("{}", value),
                );

                Inhibit(false)
            }
        });

        let temperature_slider = Scale::new(
            Orientation::Horizontal,
            Some(&Adjustment::new(100.0, 1700.0, 6500.0, 1.0, 0.0, 0.0)),
        );

        temperature_slider.connect_change_value({
            let active_bulb = Arc::clone(&active_bulb);
            move |_s, _t, value| {
                let mut value = value.round() as i32;
                println!("{}", value);
                if value > 6500 {
                    value = 6500
                }

                send_command(
                    &active_bulb.read().unwrap().location,
                    "set_ct_abx",
                    &format!("{}", value),
                );

                Inhibit(false)
            }
        });

        let brightness_label = Label::new(Some("Brightness"));
        let temperature_label = Label::new(Some("Temperature"));
        let spacer = Label::new(Some(""));

        button_row.pack_start(&brightness_label, false, false, 0);
        button_row.pack_start(&brightness_slider, false, false, 0);

        button_row.pack_start(&temperature_label, false, false, 0);
        button_row.pack_start(&temperature_slider, false, false, 0);

        button_row.pack_start(&spacer, true, true, 5);

        window.add(&button_row);
        window.show_all();
    });

    application.run();
}

fn send_command(address: &str, method: &str, params: &str) {
    let mut stream = TcpStream::connect(&address).unwrap();

    let msg = format!(
        "{{\"id\":{},\"method\":\"{}\",\"params\":[{}]}}\r\n",
        0, method, params
    );

    match stream.write(msg.as_bytes()) {
        Ok(_) => {
            print!("Message sent: {}", msg);
            stream.flush().unwrap();
        }
        Err(_) => {
            println!("Failed to send message");
            return;
        }
    }

    let mut buf = [0; 2000];

    match stream.read(&mut buf) {
        Ok(_) => {
            print!("Response: {}", str::from_utf8(&buf).unwrap());
            stream.flush().unwrap();
        }
        Err(_) => {
            println!("Failed to read response");
        }
    }
}

async fn get_bulbs(mut socket: UdpSocket) -> Vec<Bulb> {
    let mut bulbs: Vec<Bulb> = Vec::new();
    let mut responses: Vec<String> = Vec::new();

    let msg = b"M-SEARCH * HTTP/1.1\r\n
    HOST: 239.255.255.250:1982\r\n
    MAN: \"ssdp:discover\"\r\n
    ST: wifi_bulb";

    socket.send_to(msg, MULTI_CAST_ADRESS).await.unwrap();

    loop {
        let mut buf = [0; 2000];

        let valid_bytes = match time::timeout(Duration::from_secs(2), socket.recv(&mut buf)).await {
            Ok(v) => match v {
                Ok(v) => Ok(v),
                Err(e) => Err(e),
            },
            Err(_) => break,
        };

        let bytes = match valid_bytes {
            Ok(b) => b,
            Err(_) => break,
        };

        let data = &buf[..bytes];

        match str::from_utf8(data) {
            Ok(d) => responses.push(d.to_owned()),
            _ => (),
        };
    }

    println!("{}", responses.len());

    if responses.len() == 0 {
        panic!("No bulbs found")
    }

    for m in responses.into_iter() {
        let parsed_response: HashMap<&str, &str> = parse_response(&m);
        let new_bulb: Bulb = create_new_bulb(parsed_response);

        let mut duplicate: bool = false;

        for b in &bulbs {
            if b.id == new_bulb.id {
                duplicate = true;
            }
        }

        if !duplicate {
            bulbs.push(new_bulb);
        }
    }

    println!("Bulbs found: ");
    for (i, b) in bulbs.iter().enumerate() {
        println!("{}: id: {}", i, b.id);
    }

    bulbs
}

fn parse_response(message: &str) -> HashMap<&str, &str> {
    let mut lines = message.lines();
    let mut data: HashMap<&str, &str> = HashMap::new();

    loop {
        match lines.next() {
            Some(m) => {
                match parse_line(m) {
                    Some(value) => data.insert(value.0, value.1),
                    None => None,
                };
            }
            None => {
                break;
            }
        }
    }
    data
}

fn parse_line(message: &str) -> Option<(&str, &str)> {
    let m: Vec<&str> = message.splitn(2, ":").collect();

    if m.len() < 2 {
        return None;
    }

    Some((m[0].trim(), m[1].trim()))
}

fn create_new_bulb(data: HashMap<&str, &str>) -> Bulb {
    let mut new_bulb = Bulb::new_bulb();

    for (key, value) in data.into_iter() {
        println!("key: {}, value: {}", key, value);
        match key {
            "Location" => {
                let v: Vec<&str> = value.split("//").collect();
                new_bulb.location = v[1].trim().to_string();
            }
            "id" => new_bulb.id = value.to_string(),
            "power" => new_bulb.power = value.to_string(),
            "bright" => new_bulb.bright = value.to_string(),
            "color_mode" => new_bulb.color_mode = value.to_string(),
            "rgb" => new_bulb.rgb = value.to_string(),
            "ct" => new_bulb.ct = value.to_string(),
            "hue" => new_bulb.hue = value.to_string(),
            "sat" => new_bulb.sat = value.to_string(),
            "name" => new_bulb.name = value.to_string(),
            _ => (),
        }
    }

    new_bulb
}
