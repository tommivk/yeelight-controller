mod bulb;
use bulb::Bulb;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::{self};
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};

const MULTI_CAST_ADRESS: &str = "239.255.255.250:1982";

#[tokio::main]
async fn main() {
    let mut bulbs: Vec<Bulb> = Vec::new();

    let msg = b"M-SEARCH * HTTP/1.1\r\n
    HOST: 239.255.255.250:1982\r\n
    MAN: \"ssdp:discover\"\r\n
    ST: wifi_bulb";

    let mut socket = UdpSocket::bind("0.0.0.0:3480").await.unwrap();
    socket.send_to(msg, MULTI_CAST_ADRESS).await.unwrap();

    let mut responses: Vec<String> = Vec::new();

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

    loop {
        println!("Bulb number: ");

        let mut input = String::new();

        std::io::stdin()
            .read_line(&mut input)
            .expect("failed to read line");

        let bulb_number = match input.trim().parse::<u8>() {
            Ok(num) => {
                if num as usize > bulbs.len() - 1 {
                    println!("Invalid bulb number");
                    continue;
                } else {
                    num
                }
            }
            Err(_) => {
                println!("Invalid number");
                continue;
            }
        };

        let address = bulbs[bulb_number as usize].get_location();

        let mut stream = TcpStream::connect(&address).unwrap();

        let msg = format!(
            "{{\"id\":{},\"method\":\"{}\",\"params\":[\"{}\"]}}\r\n",
            0, "set_power", "off"
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

//support: get_prop set_default set_power toggle set_bright start_cf stop_cf
//set_scene cron_add cron_get cron_del set_ct_abx set_rgb set_hsv set_adjust
//adjust_bright adjust_ct adjust_color set_music set_name
