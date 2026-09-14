// 1. Clients
// 2. Gateway
// 3. Matching Engine
// 4. Drop Copy
// 5. TickerPlant
// 6. Position/Ledger
// 7. Trade Log

use std::{ collections::{ BTreeMap, HashMap }, sync::mpsc::{ self } };

use chrono::{ Duration, Utc };
use rust_decimal::dec;
use crate::{
    order::{ Order, OrderExcType, OrderStatus, OrderType },
    orderbook::OrderBook,
    trade::Trade,
};

mod order;
mod trade;
mod orderbook;
use std::thread;

fn start_trade_logger(rx: mpsc::Receiver<Trade>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // This loop blocks efficiently until a trade is received.
        // It exits automatically if the OrderBook drops its sender.
        for trade in rx {
            // Write to file, print to console, or save to DB here
            println!(
                "[Background Worker] Executed Trade #{}: Sold {} shares at ${} between Buyer {} and Seller {}",
                trade.id,
                trade.qty,
                trade.price,
                trade.buy_order_id,
                trade.sell_order_id
            );
        }
        println!("[Background Worker] Channel closed. Thread shutting down cleanly.");
    })
}

fn main() {
    let (tx, rx) = mpsc::channel::<Trade>();
    let _logg_handle = start_trade_logger(rx);
    let mut order_book = OrderBook {
        trade_counter: 0,
        asks: BTreeMap::new(),
        bids: BTreeMap::new(),
        order_index: HashMap::new(),
        trade_sender: tx, // Pass the sender here
    };
    let now = Utc::now();
    let some_order = Order {
        id: 1,
        order_type: OrderType::Bid,
        exc_type: OrderExcType::Limit,
        price: Some(dec!(52000.5)),
        qty: dec!(1.5),
        status: OrderStatus::Open,
        timestamp: now,
        expiration: now + Duration::days(1),
    };
    order_book.process_order(some_order);
}
