// 1. Clients
// 2. Gateway
// 3. Matching Engine
// 4. Drop Copy
// 5. TickerPlant
// 6. Position/Ledger
// 7. Trade Log

use std::{ cmp::Reverse, collections::{ BTreeMap, VecDeque }, sync::mpsc::Sender };

use rust_decimal::Decimal;
use crate::{ order::Order, trade::Trade };

mod order;
mod trade;
mod orderbook;
