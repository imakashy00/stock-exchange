use std::{
    fs::OpenOptions,
    io::{ BufWriter, Write },
    sync::mpsc::Receiver,
    thread::{ self, JoinHandle },
};

use crate::trade::Trade;

pub struct Logger {
    pub file_path: String,
}

impl Logger {
    pub fn new(file_path: &str) -> Self {
        Self { file_path: file_path.to_string() }
    }
    pub fn spawn_worker(&self, reciver: Receiver<Trade>) -> JoinHandle<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .expect("Failed to open trade execution log file");
        // we hav to use bufferwriter to write
        let mut writer = BufWriter::new(file);
        thread::spawn(move || {
            while let Ok(trade) = reciver.recv() {
                let log_line = format!(
                    "{},{},{},{},{},{}\n",
                    trade.id,
                    trade.buy_order_id,
                    trade.sell_order_id,
                    trade.price,
                    trade.qty,
                    trade.timestamp
                );
                // Write is a trait for writing
                if let Err(e) = writer.write_all(log_line.as_bytes()) {
                    eprintln!("Disk I/O Error writing trade: {:?}", e);
                }
                // Flush to ensure data safety on disk
                let _ = writer.flush();
            }
            println!("[Background Worker] Channel closed. Thread shutting down cleanly.");
        })
    }
}
