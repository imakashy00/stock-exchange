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
    logger::Logger,
};

mod order;
mod trade;
mod orderbook;
mod logger;

fn main() {
    let (tx, rx) = mpsc::channel::<Trade>();
    let logger = Logger::new("trade.log");
    let _logg_handle = logger.spawn_worker(rx);
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
