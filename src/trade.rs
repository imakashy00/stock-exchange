use rust_decimal::Decimal;

#[derive(Debug, PartialEq, Eq)]
pub struct Trade {
    pub buy_order_id: i32,
    pub sell_order_id: i32,
    pub price: Decimal,
    pub qty: Decimal,
    pub timestamp: i64,
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Utc;
    use rust_decimal::dec;

    #[test]
    fn trade_struct_construction() {
        let now = Utc::now();
        let trade = Trade {
            buy_order_id: 1,
            sell_order_id: 2,
            price: dec!(52000.5),
            qty: dec!(1.0),
            timestamp: now.timestamp_millis(),
        };
        assert_eq!(trade.qty, dec!(1.0));
        assert_eq!(trade.price, dec!(52000.5));
    }

    #[test]
    fn trade_ordering_trade_by_timestamp() {
        let mut trades = vec![
            Trade {
                buy_order_id: 1,
                sell_order_id: 2,
                price: dec!(10),
                qty: dec!(1),
                timestamp: 3000,
            },
            Trade {
                buy_order_id: 1,
                sell_order_id: 3,
                price: dec!(10),
                qty: dec!(1),
                timestamp: 1000,
            },
            Trade {
                buy_order_id: 2,
                sell_order_id: 4,
                price: dec!(10),
                qty: dec!(1),
                timestamp: 2000,
            }
        ];
        trades.sort_by_key(|trade| trade.timestamp);
        assert_eq!(trades[0].timestamp, 1000);
        assert_eq!(trades[1].timestamp, 2000);
        assert_eq!(trades[2].timestamp, 3000);
    }
}
