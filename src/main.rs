use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr};
use std::io::{Write, Read};
use simple_signal::{self, Signal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};
use std::fs;
use std::env;

fn wait(ms: u64) {
    let duration = time::Duration::from_millis(ms);
    thread::sleep(duration);
}

fn handle_client(mut stream: &TcpStream, buffer: Vec<u8>, file_name: &String) {
    let s: String = format!(concat!(
        "HTTP/1.1 200 OK\n",
        "Content-Type: application/octet-stream; charset=utf-8\n",
        "Content-Disposition: attachment; filename=\"{fileName}\"\n",
        "Content-Length: {length}\n\n"
    ), fileName=file_name, length=buffer.len().to_string());
    let _ = stream.write(s.as_bytes()); 
    let _ = stream.write(&buffer);
}

fn handle_file(path: &String) -> std::io::Result<Vec<u8>> {
    let mut file = fs::File::open(&path).expect("no file found");
    let metadata = fs::metadata(&path).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    file.read(&mut buffer).expect("buffer overflow");
    return Ok(buffer)
}

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    simple_signal::set_handler(&[Signal::Int, Signal::Term], move |_signals| {
        r.store(false, Ordering::SeqCst);
    });
    let args: Vec<String> = env::args().collect();
    let buffer: Vec<u8> = handle_file(&args[1])?;
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    listener.set_nonblocking(true).expect("Cannot set non-blocking");
    let mut chunks: Vec<&str> = args[1].split("/").collect();
    let file_name: String = String::from(chunks.pop().unwrap());
    println!("sharing file: {}", file_name);
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let addr: SocketAddr = s.peer_addr()?;
                println!("accepted client: {:?}", addr);
                handle_client(&s, buffer.clone(), &file_name);
                println!("ended transfer of client: {:?}", addr);
                println!("waiting 1000ms until connection shutdown");
                wait(1000);
                println!("about to shutdown");
                s.shutdown(Shutdown::Both).expect("shutdown call failed");
                println!("shutdown of client connetion: {:?}", addr);
                if !running.load(Ordering::SeqCst) { 
                    println!("\nstop listening!");
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if !running.load(Ordering::SeqCst) { 
                    println!("\nstop listening!");
                    break;
                }
            }
            Err(e) => panic!("encountered IO error: {}", e),
        }
        
    }
    Ok(())
}