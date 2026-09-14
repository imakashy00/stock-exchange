use std::{
    cmp::{ Reverse, min },
    collections::{ BTreeMap, HashMap, VecDeque },
    sync::mpsc::Sender,
};
use std::collections::btree_map::Entry;
use chrono::Utc;
use rust_decimal::{ Decimal, dec };

use crate::{ order::{ Order, OrderExcType, OrderPointer, OrderStatus, OrderType }, trade::Trade };

#[derive(Debug)]
pub struct OrderBook {
    pub trade_counter: i32,
    pub asks: BTreeMap<Decimal, VecDeque<Order>>,
    pub bids: BTreeMap<Reverse<Decimal>, VecDeque<Order>>,
    pub order_index: HashMap<i32, OrderPointer>,
    pub trade_sender: Sender<Trade>,
}

/*
    1. order aa rha h usse Orderbook me dalna h
    2. trade ho jane pr order ko tradebook se nikalna h
    3. peek the best 
    4. Unit test krna h sbka
*/
/*
    We are not passing a thread into the constructor; 
    we are accepting one end of a channel pipe (Sender). 
    This allows the main thread (running your OrderBook matching math) to instantly hand off the heavy, 
    slow logging and Drop Copy work to that single, dedicated background thread.
*/
fn execute_trade(
    trade_counter: i32,
    resting_order: &mut Order,
    incoming_order: &mut Order,
    trade_price: Decimal
) -> Trade {
    let min_trade_qty = min(resting_order.qty, incoming_order.qty);
    resting_order.qty -= min_trade_qty;
    incoming_order.qty -= min_trade_qty;

    let (buy_order_id, sell_order_id) = match incoming_order.order_type {
        OrderType::Bid => (incoming_order.id, resting_order.id),
        OrderType::Ask => (resting_order.id, incoming_order.id),
    };
    Trade {
        id: trade_counter,
        buy_order_id,
        sell_order_id,
        price: trade_price,
        qty: min_trade_qty,
        timestamp: 123456789,
    }
}

impl OrderBook {
    pub fn new(trade_sender: Sender<Trade>) -> OrderBook {
        // receives a channel capable of sending Trade data
        Self {
            trade_counter: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            order_index: HashMap::new(),
            trade_sender,
        }
    }
    pub fn insert_order(&mut self, order: Order) {
        // Only limit orders come to Orderbook market orders take whats available at the moment
        let price = order.price.expect("Limit Order must have value.");
        let order_type = order.order_type;
        if order.order_type == OrderType::Bid {
            // or_insert_with to create a Vec Deque if the price is new else push the order
            // or_insert(VecDeque::new())- slow - (method invocation confimed even if Deques exist wasting cpu cylces)
            // or_inser(VecDeque::new) - fast (passing referece of the fn)
            self.bids.entry(Reverse(price)).or_insert_with(VecDeque::new).push_back(order);
        } else {
            self.asks.entry(price).or_insert_with(VecDeque::new).push_back(order);
        }
        self.order_index.insert(order.id, OrderPointer {
            price,
            order_type,
        });
    }
    fn remove_from_entry(entry: Entry<impl Ord, VecDeque<Order>>, order_id: i32) -> Option<Order> {
        // requests the slot in the map corresponding to that price. If there is already a queue of orders at that price level (Occupied), it enters the block.
        if let Entry::Occupied(mut entry) = entry {
            let queue = entry.get_mut();
            if let Some(position) = queue.iter().position(|order| order.id == order_id) {
                let removed_order = queue.remove(position);
                if queue.is_empty() {
                    entry.remove();
                }
                return removed_order;
            }
        }
        None
    }
    // Cancel Order operation
    pub fn remove_order(&mut self, order_id: i32) -> Option<Order> {
        let pointer = self.order_index.remove(&order_id)?; // if not found returns None
        // route to the correct BTreeMap
        let order = match pointer.order_type {
            OrderType::Bid => {
                let price_key = Reverse(pointer.price); // needed to revrse because Decimal and Reverse<Decimal> are different
                Self::remove_from_entry(self.bids.entry(price_key), order_id)
            }
            OrderType::Ask => {
                let price_key = pointer.price;
                Self::remove_from_entry(self.asks.entry(price_key), order_id)
            }
        };
        order
    }
    pub fn best_bid(&self) -> Option<&Order> {
        self.bids.values().next()?.front()
    }
    pub fn best_ask(&self) -> Option<&Order> {
        self.asks.values().next()?.front()
    }
    // update order incase of partially filled or exce type (maybe)
    pub fn update_order(&mut self, order: Order) {}

    fn emit_market_partial_cancel_event(
        &mut self,
        order: &Order,
        filled_qty: Decimal,
        cancelled_qty: Decimal
    ) {}

    pub fn cancel_order(&mut self, order_id: i32) -> Option<Order> {
        let mut order = self.remove_order(order_id)?;
        order.status = OrderStatus::Canceled;
        Some(order)
    }

    pub fn process_order(&mut self, mut incoming_order: Order) -> Order {
        let original_qty: Decimal = incoming_order.qty;

        self.match_orders(&mut incoming_order); // passingref because will need order later

        if incoming_order.qty > dec!(0) {
            match incoming_order.exc_type {
                OrderExcType::Limit => {
                    self.insert_order(incoming_order);
                }
                OrderExcType::Market => {
                    let cancelled_qty: Decimal = incoming_order.qty;
                    let filled_qty = original_qty - cancelled_qty;
                    self.emit_market_partial_cancel_event(
                        &incoming_order,
                        filled_qty,
                        cancelled_qty
                    );
                }
            }
        } else {
            incoming_order.status = OrderStatus::Filled;
        }
        incoming_order
    }

    // matching engine
    fn match_orders(&mut self, incoming_order: &mut Order) {
        let mut filled_ids: Vec<i32> = Vec::new();

        match incoming_order.order_type {
            OrderType::Bid => {
                // match with available resting orders
                for (ask_price, queue) in self.asks.iter_mut() {
                    if incoming_order.exc_type == OrderExcType::Limit {
                        let limit_price = incoming_order.price.expect(
                            "Limit Order must have values."
                        );
                        if *ask_price > limit_price {
                            break;
                        }
                    }
                    // Match using fifo at this price level
                    while !queue.is_empty() && incoming_order.qty > dec!(0) {
                        let resting_order = queue.front_mut().unwrap();
                        self.trade_counter += 1;
                        let trade = execute_trade(
                            self.trade_counter,
                            resting_order,
                            incoming_order,
                            *ask_price
                        );
                        if let Err(e) = self.trade_sender.send(trade) {
                            eprintln!("Failed to send trade event to background worker: {:?}", e);
                        }

                        if resting_order.qty == dec!(0) {
                            let mut filled_order = queue.pop_front().unwrap();
                            filled_order.status = OrderStatus::Filled;
                            filled_ids.push(filled_order.id);
                        } else {
                            resting_order.status = OrderStatus::PartiallyFilled;
                        }
                    }
                }
                self.asks.retain(|_, queue| !queue.is_empty()); // removes empty queues
            }
            OrderType::Ask => {
                for (bid_price, queue) in self.bids.iter_mut() {
                    if incoming_order.exc_type == OrderExcType::Limit {
                        let limit_price = incoming_order.price.expect(
                            "Limit Order must have values."
                        );
                        // .0 unwrap the Reverse() wrapper [ *bid_price < Reverse(limit_price) ]
                        if bid_price.0 < limit_price {
                            break;
                        }
                    }
                    while !queue.is_empty() && incoming_order.qty > dec!(0) {
                        let resting_order = queue.front_mut().unwrap();
                        let trade = execute_trade(
                            self.trade_counter,
                            resting_order,
                            incoming_order,
                            bid_price.0 // .0 unwrap the reverse decimal
                        );
                        if let Err(e) = self.trade_sender.send(trade) {
                            eprintln!("Failed to send trade event to background worker: {:?}", e);
                        }

                        if resting_order.qty == dec!(0) {
                            let mut filled_order = queue.pop_front().unwrap();
                            filled_order.status = OrderStatus::Filled;
                            filled_ids.push(filled_order.id);
                        } else {
                            resting_order.status = OrderStatus::PartiallyFilled;
                        }
                    }
                }
                self.bids.retain(|_, queue| !queue.is_empty());
            }
        }
        // remove the index from the hashmap
        for id in filled_ids {
            self.order_index.remove(&id);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::order::{ OrderExcType, OrderStatus };

    use super::*;
    use chrono::Utc;
    use rust_decimal::dec;
    use std::{ cmp::Reverse, sync::mpsc };
    fn make_order(id: i32, order_type: OrderType, price: Decimal) -> Order {
        Order {
            id,
            order_type,
            exc_type: OrderExcType::Limit, // Assuming Limit variant name
            price: Some(price),
            qty: dec!(10.0),
            status: OrderStatus::Open,
            timestamp: Utc::now(),
            expiration: Utc::now(),
        }
    }
    #[test]
    fn test_insert_bid_order() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx); // Assumes an OrderBook::new() constructor}
        let price = dec!(100.5);
        let bid_order = make_order(1, OrderType::Bid, price);
        orderbook.insert_order(bid_order);

        let price = Reverse(price);
        let queue = orderbook.bids.get(&price).expect("Bid queue missing at this price lavel.");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].id, 1);
    }
    #[test]
    fn test_insert_ask_order() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);
        let price = dec!(120.5);
        let ask_order = make_order(5, OrderType::Ask, price);
        orderbook.insert_order(ask_order);

        let queue = orderbook.asks.get(&price).expect("Ask queue missing at this price.");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].id, 5);
    }
    #[test]
    #[should_panic(expected = "Limit Order must have value.")]
    fn test_insert_maket_order_panic() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);
        let market_order = Order {
            id: 5,
            order_type: OrderType::Bid,
            exc_type: OrderExcType::Market,
            price: None,
            qty: dec!(50.0),
            status: OrderStatus::Open,
            timestamp: Utc::now(),
            expiration: Utc::now(),
        };
        orderbook.insert_order(market_order);
    }
    // test remove funtion
    #[test]
    fn test_remove_bid_order() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);
        let price = dec!(100.0);

        let bid_order = make_order(10, OrderType::Bid, price);
        orderbook.insert_order(bid_order);

        let removed = orderbook.remove_order(bid_order.id);
        let removed_order = removed.expect("Expected an order to be returned.");
        assert_eq!(removed_order.id, bid_order.id);
        assert_eq!(removed_order.order_type, OrderType::Bid);

        //verify the empty orderbook
        assert!(orderbook.order_index.get(&10).is_none());
        assert!(
            orderbook.bids.get(&Reverse(dec!(100.0))).is_none(),
            "Queue should be completely removed if empty."
        )
    }
    #[test]
    fn test_remove_ask_order() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);
        let price = dec!(289.0);
        let ask_order = make_order(11, OrderType::Ask, price);
        orderbook.insert_order(ask_order);

        let removed = orderbook.remove_order(ask_order.id);
        let removed_order = removed.expect("Expected an order to be returned.");
        assert_eq!(removed_order.id, ask_order.id);
        assert_eq!(removed_order.order_type, OrderType::Ask);
        // verify the empty orderbook
        assert!(orderbook.order_index.get(&11).is_none());
        assert!(
            orderbook.asks.get(&dec!(289.0)).is_none(),
            "Queue should be completely removed if empty."
        )
    }
    #[test]
    fn test_remove_order_panic() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);
        let result = orderbook.remove_order(999);

        assert!(result.is_none(), "Removing a non-existent order must return None");
    }
    // peek tests
    #[test]
    fn test_best_bid_returns_highest_price() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);

        let low_bid = make_order(1, OrderType::Bid, dec!(100.0));
        let high_bid = make_order(2, OrderType::Bid, dec!(105.0));
        orderbook.insert_order(low_bid);
        orderbook.insert_order(high_bid);

        let best_bid = orderbook.best_bid().expect("Should return best bid");
        assert_eq!(best_bid.id, 2);
        assert_eq!(best_bid.price, Some(dec!(105.0)));
    }
    #[test]
    fn test_best_ask_returns_lowest_price() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);

        let high_ask = make_order(3, OrderType::Ask, dec!(120.0));
        let low_ask = make_order(4, OrderType::Ask, dec!(115.0));
        orderbook.insert_order(high_ask);
        orderbook.insert_order(low_ask);

        let best = orderbook.best_ask().expect("Should find a best ask.");
        assert_eq!(best.id, 4);
        assert_eq!(best.price, Some(dec!(115.0)));
    }
    #[test]
    fn test_best_prices_should_return_none() {
        let (tx, _rx) = mpsc::channel();
        let orderbook = OrderBook::new(tx);
        let best_bid = orderbook.best_bid();
        let best_ask = orderbook.best_ask();
        assert!(best_bid.is_none());
        assert!(best_ask.is_none());
    }
    #[test]
    fn test_best_bid_fifo_at_same_price() {
        let (tx, _rx) = mpsc::channel();
        let mut orderbook = OrderBook::new(tx);
        let price = dec!(100.0);
        let first_bid = make_order(10, OrderType::Bid, price);
        let second_bid = make_order(11, OrderType::Bid, price);
        orderbook.insert_order(first_bid);
        orderbook.insert_order(second_bid);

        let best_bid = orderbook.best_bid().expect("Should find a best bid.");
        assert_eq!(best_bid.id, first_bid.id);
        assert_eq!(best_bid.price, Some(price));
    }
}
