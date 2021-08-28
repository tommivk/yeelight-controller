use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::str::{self};

const MULTI_CAST_ADRESS: &str = "239.255.255.250:1982";

struct Bulb {
    location: String,
    id: String,
    power: String,
    bright: String,
    color_mode: String,
    rgb: String,
    ct: String,
    hue: String,
    sat: String,
    name: String,
}

impl Bulb {
    fn get_id(&self) -> &String {
        &self.id
    }
    fn get_location(&self) -> &String {
        &self.location
    }
    fn new_bulb() -> Bulb {
        Bulb {
            location: String::new(),
            id: String::new(),
            power: String::new(),
            bright: String::new(),
            color_mode: String::new(),
            rgb: String::new(),
            ct: String::new(),
            hue: String::new(),
            sat: String::new(),
            name: String::new(),
        }
    }
}

fn main() {
    let mut bulbs: Vec<Bulb> = Vec::new();

    let socket = UdpSocket::bind("0.0.0.0:3480").expect("Failed to bind adress");

    let msg = b"M-SEARCH * HTTP/1.1\r\n
    HOST: 239.255.255.250:1982\r\n
    MAN: \"ssdp:discover\"\r\n
    ST: wifi_bulb";

    socket
        .send_to(msg, MULTI_CAST_ADRESS)
        .expect("Failed to send message to multicast address");

    let mut buf = [0; 2000];

    let message: Result<&str, Box<dyn Error>> = match socket.recv(&mut buf) {
        Ok(received) => match str::from_utf8(&mut buf[..received]) {
            Ok(m) => Ok(m),
            Err(e) => Err(Box::new(e)),
        },
        Err(e) => Err(Box::new(e)),
    };

    let parsed_data: HashMap<&str, &str> = parse_response(&message.unwrap());

    let new_bulb: Bulb = parse_values(parsed_data);

    bulbs.push(new_bulb);

    let address = bulbs[0].get_location();

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

fn parse_values(data: HashMap<&str, &str>) -> Bulb {
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
