use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// ============================================================================
// Scenario API State Structs
// These mirror the Python classes in ACEBench/model_inference/multi_turn/scenariosen/
// ============================================================================

/// Base API state - shared by MessageApi, ReminderApi, FoodPlatform
/// Python: scenariosen/phone_platform/base_api.py
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BaseApi {
    pub wifi: bool,
    pub logged_in: bool,
}

/// User information for MessageApi
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageUser {
    pub user_id: String,
    pub phone_number: String,
    pub occupation: String,
}

/// Message record in inbox
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub sender_id: String,
    pub receiver_id: String,
    pub message: String,
    pub time: Option<String>, // Optional - new messages may not have time
}

/// Message API state
/// Python: scenariosen/phone_platform/message.py
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageApi {
    // Inherited from BaseApi
    pub wifi: bool,
    pub logged_in: bool,
    // MessageApi specific fields
    pub max_capacity: u32,
    pub user_list: IndexMap<String, MessageUser>, // key: user name (e.g., "Eve")
    pub inbox: IndexMap<u32, Message>,            // key: message_id
    pub message_id_counter: u32,
}

/// Reminder record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reminder {
    pub reminder_id: u32,
    pub title: String,
    pub description: String,
    pub time: String, // format: "YYYY-MM-DD HH:MM"
    pub notified: bool,
}

/// Reminder API state
/// Python: scenariosen/phone_platform/reminder.py
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReminderApi {
    // Inherited from BaseApi
    pub wifi: bool,
    pub logged_in: bool,
    // ReminderApi specific fields
    pub max_capacity: u32,
    pub reminder_list: IndexMap<u32, Reminder>, // key: internal id (1, 2, 3...)
    pub reminder_id_counter: u32,
}

/// Food platform user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoodUser {
    pub user_id: String,
    pub password: String,
    pub balance: f64,
}

/// Menu item for a merchant
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MenuItem {
    pub product: String,
    pub price: f64,
}

/// Merchant on the food platform
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Merchant {
    pub merchant_id: String,
    pub service_type: String,
    pub menu: Vec<MenuItem>,
}

/// Order item
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderItem {
    pub product: String,
    pub quantity: u32,
    pub price_per_unit: f64,
}

/// Food delivery order
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoodOrder {
    pub user_name: String,
    pub merchant_name: String,
    pub items: Vec<OrderItem>,
    pub total_price: f64,
}

/// Food Platform API state
/// Python: scenariosen/phone_platform/food_services.py
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoodPlatform {
    // Inherited from BaseApi
    pub wifi: bool,
    pub logged_in: bool,
    // FoodPlatform specific fields
    pub users: IndexMap<String, FoodUser>,       // key: user name (e.g., "Eve")
    pub merchant_list: IndexMap<String, Merchant>, // key: merchant name (e.g., "Domino's")
    pub logged_in_users: Vec<String>,            // list of logged-in usernames
    pub orders: Vec<FoodOrder>,
}

/// Travel system user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TravelUser {
    pub user_name: String,
    pub password: String,
    pub cash_balance: f64,
    pub bank_balance: f64,
    pub membership_level: String, // "regular", "silver", "gold"
}

/// Flight information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flight {
    pub flight_no: String,
    pub origin: String,
    pub destination: String,
    pub depart_time: String,  // format: "YYYY-MM-DD HH:MM:SS"
    pub arrival_time: String,
    pub status: String, // "available"
    pub seats_available: u32,
    pub economy_price: u32,
    pub business_price: u32,
}

/// Flight reservation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reservation {
    pub reservation_id: String,
    pub user_id: String,
    pub flight_no: String,
    pub payment_method: String, // "cash" or "bank"
    pub cabin: String,          // "Economy Class" or "Business Class"
    pub baggage: u32,
    pub origin: String,
    pub destination: String,
}

/// Travel API state (does NOT inherit from BaseApi)
/// Python: scenariosen/travel.py
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Travel {
    pub users: IndexMap<String, TravelUser>, // key: user_id (e.g., "user1")
    pub flights: Vec<Flight>,
    pub reservations: Vec<Reservation>,
}

/// Unified world state for multi-turn/multi-step scenarios
/// Contains the state of all involved API instances
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldState {
    #[serde(rename = "BaseApi", default)]
    pub base_api: Option<BaseApi>,
    #[serde(rename = "MessageApi", default)]
    pub message_api: Option<MessageApi>,
    #[serde(rename = "ReminderApi", default)]
    pub reminder_api: Option<ReminderApi>,
    #[serde(rename = "FoodPlatform", default)]
    pub food_platform: Option<FoodPlatform>,
    #[serde(rename = "Travel", default)]
    pub travel: Option<Travel>,
}
