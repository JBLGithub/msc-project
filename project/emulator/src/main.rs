mod services;
mod models;
mod layers;

use modular_bitfield_msb::bitfield;
use modular_bitfield_msb::prelude::{B16, B64};
use rand::rngs::StdRng;
use services::log_services::log_info;
use tokio::io::{self, AsyncBufReadExt};
use tokio::time::{self, sleep};
use tokio::{signal, time::Instant};
use std::io::Write;
use std::time::Duration;
use chrono::Utc;
use rand::{Rng, SeedableRng};

// for logger only - not supposed to be used directly
use layers::overlay_network::CONFIG;
use layers::underlay_network::{close_underlay_socket, open_underlay_socket};

// import JTP protocol
use crate::layers::jtp_network::{close_jtp_socket, jtp_rx, jtp_nid_tx, jtp_fqdn_tx, open_jtp_socket };

#[tokio::main]
async fn main() {

    if CONFIG.app.logger {
        logger_app().await;
    } 
    else if CONFIG.app.test_convergence {
        convergence_test().await;
    }
    else if CONFIG.app.test_single {
        packet_overhead_single_test().await;
    }
    else if CONFIG.app.test_flow {
        packet_overhead_flow_test().await;
    }
    else if CONFIG.app.test_throughput {
        throughput_test().await;
    }
    else if CONFIG.app.test_latency {
        rtt_test().await;
    } else if CONFIG.app.sensor_application {
        sensor_application().await;
    }
    else {
        user_app().await;
    }
    
}



async fn convergence_test()
{

    match open_jtp_socket().await {
        Ok(emulator_socket) => {

            println!("\x1B[2J\x1B[1;1H");
            sleep(Duration::from_secs(5)).await;

            // buffer / interval
            let buffer = vec![0x44; 8 as usize];
            let mut interval = time::interval(Duration::from_secs(5));

            // 50 measurements
            for i in 0..50 {

                interval.tick().await;
                let _ = jtp_fqdn_tx(&emulator_socket, &"node2".to_string(), &buffer).await;
                println!("Packet {} sent", i+1);

            }

            signal::ctrl_c().await.expect("failed to listen for ctrl+c signal");
            close_jtp_socket(emulator_socket).await.unwrap();

        },
        Err(err) => {
            eprintln!("** error: {}", err);
        }
    }

}

async fn packet_overhead_single_test()
{

    match open_jtp_socket().await {
        Ok(emulator_socket) => {

            println!("\x1B[2J\x1B[1;1H");
            sleep(Duration::from_secs(5)).await;

            // buffer / interval
            let buffer = vec![0x44; 8 as usize];
            let mut interval = time::interval(Duration::from_secs(5));

            // 30 measurements
            for i in 0..30 {

                // wait
                interval.tick().await;
                let _ = jtp_fqdn_tx(&emulator_socket, &"node2".to_string(), &buffer).await;
                println!("Packet {} sent", i+1);

            }

            signal::ctrl_c().await.expect("failed to listen for ctrl+c signal");
            close_jtp_socket(emulator_socket).await.unwrap();

        },
        Err(err) => {
            eprintln!("** error: {}", err);
        }
    }
}

async fn packet_overhead_flow_test()
{

    match open_jtp_socket().await {
        Ok(emulator_socket) => {

            println!("\x1B[2J\x1B[1;1H");
            sleep(Duration::from_secs(5)).await;

            // buffer / packets 10Mbps
            let buffer = vec![0x44; CONFIG.network.MTU as usize];
            let nb_packets = (10 * 1_000_000) / (CONFIG.network.MTU as usize * 8);

            // interval
            let mut interval = time::interval(Duration::from_secs(5));

            for i in 0..30 {

                // wait to reset TTL
                interval.tick().await;
                let start_time = Instant::now();

                for _ in 0..nb_packets {
                    let _ = jtp_fqdn_tx(&emulator_socket, &"node2".to_string(), &buffer).await;
                }

                println!("{}: Packet {} sent", i+1, nb_packets);

                // check if flow is 10Mbps
                let elapsed = start_time.elapsed();
                if elapsed > Duration::from_secs(1) {
                    println!("exceeded");
                };

            }

            signal::ctrl_c().await.expect("failed to listen for ctrl+c signal");
            close_jtp_socket(emulator_socket).await.unwrap();

        },
        Err(err) => {
            eprintln!("** error: {}", err);
        }
    }
}

async fn throughput_test()
{

    match open_jtp_socket().await {
        Ok(emulator_socket) => {

            println!("\x1B[2J\x1B[1;1H");
            println!("waiting");
            sleep(Duration::from_secs(60)).await;
            println!("starting");

            if CONFIG.node.name == "node1" {

                let buffer = vec![0x44; CONFIG.network.MTU as usize];

                let start_time = Instant::now();
                let loop_duration = Duration::from_secs(30);

                while start_time.elapsed() < loop_duration {

                    match jtp_fqdn_tx(&emulator_socket, &"node2".to_string(), &buffer).await {
                        Ok(()) => {},
                        Err(_) => {}
                    }

                }

            }

            println!("done");

            signal::ctrl_c().await.expect("failed to listen for ctrl+c signal");
            close_jtp_socket(emulator_socket).await.unwrap();

        },
        Err(_) => {
            eprintln!("** failed to open socket");
        }
    }

}

async fn rtt_test() 
{

    match open_jtp_socket().await {
        Ok(emulator_socket) => {

            #[bitfield]
            struct RTTPckHeader {
                timestamp: B64,
                sequence: B16
            }

            println!("\x1B[2J\x1B[1;1H");
            sleep(Duration::from_secs(20)).await;
            let mut counter:  u16 = 0;

            for i in 0u16..2000 {
                if CONFIG.node.name == "node1" {

                    match jtp_rx(-1).await {
                        Ok(response) => {

                            println!("packet received");
                            match jtp_fqdn_tx(&emulator_socket, &"node2".to_string(), &response.payload).await {
                                Ok(()) => {
                                    counter += 1;
                                    println!("packet sent: {}", counter);
                                },
                                Err(_) => {}
                            }

                        },
                        Err(_) => {}
                    }
                }

                else if CONFIG.node.name == "node2" {

                    let mut rtt_header = RTTPckHeader::new();
            
                    // measure time
                    let now = Utc::now();
                    let now_timestamp = now.timestamp_micros() as u64;
                    rtt_header.set_timestamp(now_timestamp);
                    rtt_header.set_sequence(counter);
                    let mut buffer = rtt_header.into_bytes().to_vec();

                    let payload = vec![0x44; 8 as usize];
                    buffer.extend(payload);
                    
                    match jtp_fqdn_tx(&emulator_socket, &"node1".to_string(), &buffer).await {
                        Ok(()) => {

                            println!("packet sent");
                            match jtp_rx(5000).await {
                                Ok(response) => {

                                    let now = Utc::now();
                                    counter += 1;
                                    println!("packet received: {}", counter);

                                    match response.payload[0..10].try_into() as Result<[u8; 10], _> {
                                        Ok(response) => {

                                            let rtt_header_response = RTTPckHeader::from_bytes(response);
                                            let sequence = rtt_header_response.sequence();

                                            if sequence == i {
                                                         
                                                let elapsed = now.timestamp_micros() as u64 - rtt_header_response.timestamp();
                                                log_info(&emulator_socket, &format!("METRIC;1;{}", elapsed)).await;

                                            }

                                            sleep(Duration::from_millis(10)).await;

                                        },
                                        Err(err) => {
                                            eprintln!("** failed to deserialise packet: {:?}", err);
                                        }
                                    }

                                },
                                Err(_) => {}
                            }

                        },
                        Err(_) => {}
                    }

                }

            }

            signal::ctrl_c().await.expect("failed to listen for ctrl+c signal");
            close_jtp_socket(emulator_socket).await.unwrap();

        },
        Err(_) => {
            eprintln!("** failed to open socket");
        }
    }

}



#[bitfield]
struct SensorPacket {
    temperature: B64,
    humidity: B64,
    soil_mositure: B64,
}

fn fake_metric(current: f64, step_range: &Vec<f64>, upwards: &mut bool)
    -> f64
{
    if *upwards {
        let now = current + step_range[2];
        if now >= step_range[1] {
            *upwards = false;
        }
        now
    } else {
        let now = current - step_range[2];
        if now <= step_range[0] {
            *upwards = true;
        }
        now
    }
}

// modify for each topolopy
fn closest_sink(node_name: &String)
    -> (String, String) 
{
    let num_s = &node_name[4..];
    let num: u64 = match num_s.parse() {
        Ok(n) => {n},
        Err(_) => 3
    };

    // TOPOLOGY 1, 5, 7, 8
    if num > 7 {
        ("node2".to_string(), "node1".to_string())
    } else {
        ("node1".to_string(), "node2".to_string())
    }

    // TOPOLOGY 2
    // if num == 6 || num == 7 || num == 8 || num == 9 || num == 10 {
    //     ("node2".to_string(), "node1".to_string())
    // } else {
    //     ("node1".to_string(), "node2".to_string())
    // }

    // TOPOLOGY 3
    // if num == 4 || num == 5 || num == 6 || num == 7 || num == 8 {
    //     ("node1".to_string(), "node2".to_string())
    // } else {
    //     ("node2".to_string(), "node1".to_string())
    // }

    // TOPOLOGY 4
    // if num > 8 {
    //     ("node2".to_string(), "node1".to_string())
    // } else {
    //     ("node1".to_string(), "node2".to_string())
    // }

    // TOPOLOGY 6
    // if num > 10 {
    //     ("node2".to_string(), "node1".to_string())
    // } else {
    //     ("node1".to_string(), "node2".to_string())
    // }


}

async fn sensor_application() {

    match open_jtp_socket().await {
        Ok(emulator_socket) => {

            if CONFIG.node.name == "node1" || CONFIG.node.name == "node2" {

                loop {

                    tokio::select! {
                        _ = signal::ctrl_c() => {
                            println!("ctrl+c exiting...");
                            break;
                        }
                        response = jtp_rx(-1) => {
                            match response {
                                Ok(response) => {

                                    match response.payload.try_into() as Result<[u8; 24], _> {
                                        Ok(data) => {

                                            let s_packet = SensorPacket::from_bytes(data);
                                            let current_temp = f64::from_bits(s_packet.temperature());
                                            let current_humi = f64::from_bits(s_packet.humidity());
                                            let current_soil = f64::from_bits(s_packet.soil_mositure());

                                            print!("TOP;9");
                                            print!(";SINK;{}", CONFIG.node.name);
                                            print!(";DEVICE;{}", response.source_nid);
                                            print!(";temperature;{:.1}", current_temp);
                                            print!(";humidity;{:.1}", current_humi);
                                            println!(";soil;{:.1}", current_soil);

                                        },
                                        Err(_) => {
                                            println!("** received corrupted measurements");
                                            continue;
                                        }
                                    }

                                },
                                Err(_) => {
                                    println!("** failed to receive measurements")
                                }
                            }
                            
                        }

                    }
                }

                close_jtp_socket(emulator_socket).await.unwrap();

            }
            else {

                let temp_range_steps: Vec<f64> = vec![-10.0, 40.0, 5.0];
                let humi_range_steps: Vec<f64> = vec![10.0, 90.0, 10.0];
                let soil_range_steps: Vec<f64> = vec![10.0, 90.0, 5.0];

                let mut rng = StdRng::from_entropy();
                
                // the sensors send at different times same interval
                let random_time = rng.gen_range(0..=5);
                sleep(Duration::from_secs(random_time+10)).await;

                let random_temp = rng.gen_range(temp_range_steps[0]..temp_range_steps[1]);
                let mut current_temp = (random_temp / temp_range_steps[2]).round() * temp_range_steps[2];

                let random_humi = rng.gen_range(humi_range_steps[0]..humi_range_steps[1]);
                let mut current_humi = (random_humi / humi_range_steps[2]).round() * humi_range_steps[2];
                
                let random_soil = rng.gen_range(soil_range_steps[0]..soil_range_steps[1]);
                let mut current_soil = (random_soil / soil_range_steps[2]).round() * soil_range_steps[2];

                let mut upwards_temp = true;
                let mut upwards_humi = true;
                let mut upwards_soil = true;

                let mut interval = time::interval(Duration::from_secs(5));
                for _ in 0..20 {

                    interval.tick().await;

                    current_temp = fake_metric(current_temp, &temp_range_steps, &mut upwards_temp);
                    current_humi = fake_metric(current_humi, &humi_range_steps, &mut upwards_humi);
                    current_soil = fake_metric(current_soil, &soil_range_steps, &mut upwards_soil);

                    let s_packet = SensorPacket::new()
                        .with_temperature(current_temp.to_bits())
                        .with_humidity(current_humi.to_bits())
                        .with_soil_mositure(current_soil.to_bits())
                        .into_bytes();

                    match jtp_fqdn_tx(&emulator_socket, &closest_sink(&CONFIG.node.name).0, &s_packet).await {
                        Ok(()) => {
                            println!("** measurement sent to node1");
                        },
                        Err(_) => {

                            match jtp_fqdn_tx(&emulator_socket, &closest_sink(&CONFIG.node.name).1, &s_packet).await {
                                Ok(()) => {
                                    println!("** measurement sent to node2");
                                },
                                Err(err) => {
                                    println!("** failed to send measurement: {}", err);
                                }
                            }

                        }
                    }

                }

                println!("Done");
                signal::ctrl_c().await.expect("failed to listen for ctrl+c signal");
                close_jtp_socket(emulator_socket).await.unwrap();

            }

        },
        Err(_) => {
            eprintln!("** failed to open socket");
        }
    }

}



async fn logger_app()
{
    match open_underlay_socket().await {
        Ok(emulator_socket) => {

            println!("\x1B[2J\x1B[1;1H");

            let mut buf = [0; 1024];
            loop {
                tokio::select! {
                    result = emulator_socket.mulcast_socket.recv_from(&mut buf) => {
                        match result {
                            Ok((len, addr)) => {

                                match std::str::from_utf8(&buf[..len]) {
                                    Ok(log) => {
                                        println!("{}", log);
                                    },
                                    Err(err) => {
                                        let string = String::from_utf8_lossy(&buf);
                                        println!("{}, {:?}", string, addr);
                                        for byte in buf {
                                            print!("{:02x} ", byte);
                                        }
                                        println!();
                                        eprintln!("** - failed to convert log to string: {}", err);
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("** - failed to receive log: {}", err);
                            }
                        }
                    },
                    _ = signal::ctrl_c() => {
                        println!("** - ctrl+c received, exiting");
                        break;
                    },
                }
            }

            let _ = close_underlay_socket(emulator_socket).await;
        },
        Err(err) => {
            eprintln!("** - failed to setup logger: {}", err);
        }
    }
}



async fn user_app()
{
    // bind socket to interface
    match open_jtp_socket().await {
        Ok(emulator_socket)  => {

            print!("\x1B[2J\x1B[1;1H");
            println!("** - successfully opened socket");

            // create an asynchronous line reader from stdin
            let stdin = io::stdin();
            let reader = io::BufReader::new(stdin);
            let mut lines = reader.lines();

            // loop states
            let mut destination_nid:u64 = 0;
            let mut destination_fqdn = "".to_string();
            let mut text_state = 0;

            loop {

                if text_state == 0 {
                    print!("Enter a destination NID or FQDN to send a message: ");
                }
                else if text_state == 1 {
                    print!("Enter message to send to {} (hex: 0x{:016X}): ", destination_nid, destination_nid);
                } 
                else {
                    print!("Enter message to send to {}: ", destination_fqdn);
                }
                std::io::stdout().flush().unwrap();

                tokio::select! {

                    // blocking receiver
                    packet_result = jtp_rx(-1) => {
                        println!();
                        match packet_result {
                            Ok(packet) => {
                                match String::from_utf8(packet.payload) {
                                    Ok(message) => {
                                        println!("Received from 0x{:016X}: {}", packet.source_nid, message);
                                    },
                                    Err(err) => {
                                        eprintln!("** - error: {}", err);
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("** - error: {}", err);
                            }
                        }
                    }

                    _ = signal::ctrl_c() => {
                        println!();
                        println!("** - ctrl+c received, exiting");
                        break;
                    }

                    line = lines.next_line() => {
                        match line {
                            Ok(Some(input)) => {    

                                if text_state == 0 {

                                    match u64::from_str_radix(&input, 16) {
                                        Ok(dest_nid) => {
                                            destination_nid = dest_nid;
                                            text_state = 1;
                                        },
                                        Err(_) => {
                                            destination_fqdn = input;
                                            text_state = 2;
                                        }
                                    }

                                }
                                else if text_state == 1 {
                                    let _ = jtp_nid_tx(&emulator_socket, &destination_nid, input.as_bytes()).await;
                                    text_state = 0;
                                }
                                else {
                                    let _ = jtp_fqdn_tx(&emulator_socket, &destination_fqdn, input.as_bytes()).await;
                                    text_state = 0;
                                }
                            }
                            Ok(None) => {
                                eprintln!("** - error: eof reached");
                                break;
                            }
                            Err(err) => {
                                println!("** - error: reading line: {}", err);
                                break;
                            }
                        }
                    }

                }

            }

            // leave the multicast networks
            match close_jtp_socket(emulator_socket).await {
                Ok(()) => {
                    println!("** - successfully closed socket");
                },
                Err(err) => {
                    eprintln!("** - error: closing socket: {}", err);
                }
            }
        }, 
        Err(err) => {
            eprintln!("** - error: opening socket: {}", err);
        }
    }
}