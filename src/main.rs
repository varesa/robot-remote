use std::thread;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use websocket::{sync::Server,OwnedMessage};
use serialport::prelude::*;

const PROTO: &str = "robot-control";

fn main() {
    let serial_serialport_name = "/dev/serial0";
    let mut serial_serialport_settings: SerialPortSettings = Default::default();

    let port = serialport::open_with_settings(&serial_serialport_name, &serial_serialport_settings).unwrap();
    let serialport_shared = Arc::new(Mutex::new(port));

    let server = Server::bind("0.0.0.0:2794").unwrap();

    // Server is an iterator that waits for incoming connections
    for request in server.filter_map(Result::ok) {
        let serialport_local = Arc::clone(&serialport_shared);
        thread::spawn(move || {
            if !request.protocols().contains(&PROTO.to_string()) {
                println!("Rejecting due to no supported protocols, {:?}", request.protocols());
                request.reject().unwrap();
                return;
            }

            let mut client = request.use_protocol(PROTO).accept().unwrap();
            let ip = client.peer_addr().unwrap();
            println!("Connection from {}", ip);

            let message = OwnedMessage::Text("Welcome".to_string());
            client.send_message(&message).unwrap();

            let (mut receiver, mut sender) = client.split().unwrap();

            for message in receiver.incoming_messages() {
                let message = message.unwrap();
                println!("{:?}", &message);

                match message {
                    OwnedMessage::Close(_) => {
                        let message = OwnedMessage::Close(None);
                        sender.send_message(&message).unwrap();
                        println!("Client {} disconnected", ip);
                        return;
                    }
                    OwnedMessage::Ping(ping) => {
                        let message = OwnedMessage::Pong(ping);
                        sender.send_message(&message).unwrap();
                    }
                    OwnedMessage::Binary(data) => {
                        println!("Command: 0x{:x}", &data[0]);
                        serialport_local.lock().unwrap().write(&data);
                        
                    }
                    _ => sender.send_message(&message).unwrap(),
                }
            }
        });
    }
}
