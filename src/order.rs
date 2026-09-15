use chrono::{ DateTime, Utc };
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderExcType {
    Limit,
    Market,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
    Rejected,
}
#[derive(Debug)]
pub struct OrderPointer {
    pub price: Decimal,
    pub order_type: OrderType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub id: i32,
    pub order_type: OrderType,
    pub exc_type: OrderExcType,
    pub price: Option<Decimal>, // Optional because it may be market(None) or limit(Some) order
    pub qty: Decimal,
    pub status: OrderStatus,
    pub timestamp: DateTime<Utc>,
    pub expiration: DateTime<Utc>,
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::{ Duration, TimeZone, Utc };
    use rust_decimal::dec;
    #[test]
    fn order_struct_construction() {
        let now = Utc::now();
        let order = Order {
            id: 1,
            order_type: OrderType::Bid,
            exc_type: OrderExcType::Limit,
            price: Some(dec!(52000.5)),
            qty: dec!(1.5),
            status: OrderStatus::Open,
            timestamp: now,
            expiration: now + Duration::days(1),
        };
        assert_eq!(order.id, 1);
        assert_eq!(order.price, Some(dec!(52000.5)));
    }

    #[test]
    fn test_ordering_orders_by_timestamp() {
        let base_time = Utc.with_ymd_and_hms(2026, 09, 13, 21, 0, 0).unwrap();
        let early_time = base_time - Duration::minutes(10);
        let late_time = base_time + Duration::minutes(10);

        let make_order = |id: i32, time: DateTime<Utc>| Order {
            id: id,
            order_type: OrderType::Ask,
            exc_type: OrderExcType::Market,
            price: Some(dec!(12.5)),
            qty: dec!(3.5),
            status: OrderStatus::Open,
            timestamp: time,
            expiration: time + Duration::days(1),
        };
        let mut orders = vec![
            make_order(3, late_time),
            make_order(1, early_time),
            make_order(2, base_time)
        ];
        orders.sort_by_key(|order| order.timestamp);
        assert_eq!(orders[0].id, 1, "Oldest order should be first");
        assert_eq!(orders[1].id, 2, "Middle order should be second");
        assert_eq!(orders[2].id, 3, "Newest order should be last");
    }
}
