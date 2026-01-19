use std::cell::RefCell;

use indexmap::IndexMap;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::base_api::{BaseApi, ExecutionResult};

/// Food platform user
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoodUser {
    pub user_id: String,
    pub password: String,
    pub balance: NotNan<f64>,
}

/// Menu item for a merchant
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuItem {
    pub product: String,
    pub price: NotNan<f64>,
}

/// Merchant on the food platform
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Merchant {
    pub merchant_id: String,
    pub service_type: String,
    pub menu: Vec<MenuItem>,
}

/// Order item
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderItem {
    pub product: String,
    pub quantity: u32,
    pub price_per_unit: NotNan<f64>,
}

fn default_quantity() -> u32 {
    1
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArgumentItem {
    pub product: String,
    #[serde(default = "default_quantity")]
    pub quantity: u32,
}

/// Food delivery order
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoodOrder {
    pub user_name: String,
    pub merchant_name: String,
    pub items: Vec<OrderItem>,
    pub total_price: NotNan<f64>,
}

/// Food Platform API state
/// Python: scenariosen/phone_platform/food_services.py
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FoodPlatform {
    pub base_api: BaseApi,
    // FoodPlatform specific fields
    pub users: IndexMap<String, FoodUser>, // key: user name (e.g., "Eve")
    pub merchant_list: IndexMap<String, Merchant>, // key: merchant name (e.g., "Domino's")
    pub logged_in_users: Vec<String>,      // list of logged-in usernames
    pub orders: Vec<FoodOrder>,
}


#[derive(Clone, Deserialize)]
pub struct LoginFoodPlatformArgs {
    pub username: String,
    pub password: String,
}
#[derive(Clone, Deserialize)]
pub struct CheckBalanceArgs {
    pub user_name: String,
}
#[derive(Clone, Deserialize)]
pub struct AddFoodDeliveryOrderArgs {
    pub username: String,
    pub merchant_name: String,
    pub items: Vec<ArgumentItem>, // (product_name, quantity)
}
#[derive(Clone, Deserialize)]
pub struct GetProductArgs {
    pub merchant_name: String,
}
#[derive(Clone, Deserialize)]
pub struct ViewOrdersArgs {
    pub user_name: String,
}
#[derive(Clone, Deserialize)]
pub struct SearchOrdersArgs {
    pub keyword: String,
}
impl Default for FoodPlatform {
    fn default() -> Self {
        let users: IndexMap<String, FoodUser> = vec![
            (
                "Eve".to_string(),
                FoodUser {
                    user_id: "U100".to_string(),
                    password: "password123".to_string(),
                    balance: NotNan::new(500.0).unwrap(),
                },
            ),
            (
                "Frank".to_string(),
                FoodUser {
                    user_id: "U101".to_string(),
                    password: "password456".to_string(),
                    balance: NotNan::new(300.0).unwrap(),
                },
            ),
            (
                "Grace".to_string(),
                FoodUser {
                    user_id: "U102".to_string(),
                    password: "password789".to_string(),
                    balance: NotNan::new(150.0).unwrap(),
                },
            ),
            (
                "Helen".to_string(),
                FoodUser {
                    user_id: "U103".to_string(),
                    password: "password321".to_string(),
                    balance: NotNan::new(800.0).unwrap(),
                },
            ),
            (
                "Isaac".to_string(),
                FoodUser {
                    user_id: "U104".to_string(),
                    password: "password654".to_string(),
                    balance: NotNan::new(400.0).unwrap(),
                },
            ),
            (
                "Jack".to_string(),
                FoodUser {
                    user_id: "U105".to_string(),
                    password: "password654".to_string(),
                    balance: NotNan::new(120.0).unwrap(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let merchant_list: IndexMap<String, Merchant> = vec![
            (
                "Domino's".to_string(),
                Merchant {
                    merchant_id: "M100".to_string(),
                    service_type: "Pizza".to_string(),
                    menu: vec![
                        MenuItem {
                            product: "Margherita Pizza".to_string(),
                            price: NotNan::new(68.0).unwrap(),
                        },
                        MenuItem {
                            product: "Super Supreme Pizza".to_string(),
                            price: NotNan::new(88.0).unwrap(),
                        },
                    ],
                },
            ),
            (
                "Rice Village Bibimbap".to_string(),
                Merchant {
                    merchant_id: "M101".to_string(),
                    service_type: "Bibimbap".to_string(),
                    menu: vec![
                        MenuItem {
                            product: "Stone Pot Bibimbap".to_string(),
                            price: NotNan::new(35.0).unwrap(),
                        },
                        MenuItem {
                            product: "Korean Beef Bibimbap".to_string(),
                            price: NotNan::new(45.0).unwrap(),
                        },
                    ],
                },
            ),
            (
                "Haidilao".to_string(),
                Merchant {
                    merchant_id: "M102".to_string(),
                    service_type: "Hotpot".to_string(),
                    menu: vec![
                        MenuItem {
                            product: "Beef Rolls".to_string(),
                            price: NotNan::new(68.0).unwrap(),
                        },
                        MenuItem {
                            product: "Seafood Platter".to_string(),
                            price: NotNan::new(88.0).unwrap(),
                        },
                    ],
                },
            ),
            (
                "Heytea".to_string(),
                Merchant {
                    merchant_id: "M103".to_string(),
                    service_type: "Milk Tea".to_string(),
                    menu: vec![
                        MenuItem {
                            product: "Cheese Milk Tea".to_string(),
                            price: NotNan::new(25.0).unwrap(),
                        },
                        MenuItem {
                            product: "Four Seasons Spring Milk Tea".to_string(),
                            price: NotNan::new(22.0).unwrap(),
                        },
                    ],
                },
            ),
            (
                "Hema Fresh".to_string(),
                Merchant {
                    merchant_id: "M104".to_string(),
                    service_type: "Fresh Grocery".to_string(),
                    menu: vec![
                        MenuItem {
                            product: "Organic Vegetable Pack".to_string(),
                            price: NotNan::new(15.0).unwrap(),
                        },
                        MenuItem {
                            product: "Fresh Gift Pack".to_string(),
                            price: NotNan::new(99.0).unwrap(),
                        },
                    ],
                },
            ),
            (
                "Jiutian BBQ".to_string(),
                Merchant {
                    merchant_id: "M105".to_string(),
                    service_type: "BBQ".to_string(),
                    menu: vec![
                        MenuItem {
                            product: "Korean Grilled Beef".to_string(),
                            price: NotNan::new(128.0).unwrap(),
                        },
                        MenuItem {
                            product: "Grilled Pork Belly".to_string(),
                            price: NotNan::new(78.0).unwrap(),
                        },
                    ],
                },
            ),
        ]
        .into_iter()
        .collect();
        let logged_in_users: Vec<String> = Vec::new();
        let orders: Vec<FoodOrder> = Vec::new();
        FoodPlatform {
            base_api: BaseApi::default(),
            users,
            merchant_list,
            logged_in_users,
            orders,
        }
    }
}

impl FoodPlatform {
    pub fn login_food_platform(&mut self, username: String, password: String) -> ExecutionResult {
        if !self.base_api.wifi {
            return ExecutionResult::error("Wi-Fi is not enabled, unable to login".to_string());
        }
        let Some(user) = self.users.get(&username) else {
            return ExecutionResult::error("User does not exist".to_string());   
        };
        if user.password != password {
            return ExecutionResult::error("Incorrect password".to_string());
        }
        // Check if the user is already logged in
        if self.logged_in_users.contains(&username.to_string()) {
            return ExecutionResult::error(format!("{} is already logged in", username));
        }
        // Record the logged-in user
        self.logged_in_users.push(username.to_string());
        ExecutionResult::success(format!("User {} has successfully logged in!", username))
    }
    pub fn view_logged_in_users(&self) -> ExecutionResult {
        if self.logged_in_users.is_empty() {
            return ExecutionResult::error("No users are currently logged in to the food platform".to_string());
        }
        ExecutionResult::success(format!("Logged in users: {:?}", self.logged_in_users))
    }
    // unify the return type to ExecutionResult, unlike the original implementation
    // this is much easier to handle, and does not affect the functionality much
    pub fn check_balance(&self, user_name: String) -> ExecutionResult {
        match self.users.get(&user_name) {
            Some(user) => ExecutionResult::success(format!("User {} has a balance of {}", user_name, user.balance)),
            None => ExecutionResult::error(format!("User {} does not exist", user_name)),
        }
    }

    pub fn add_food_delivery_order(
        &mut self,
        username: String,
        merchant_name: String,
        items: Vec<ArgumentItem>, // (product_name, quantity)
    ) -> ExecutionResult {
        if !self.logged_in_users.contains(&username.to_string()) {
            return ExecutionResult::error(format!("User {} is not logged in to the food platform", username));
        }
        let Some(merchant) = self.merchant_list.get(&merchant_name) else {
            return ExecutionResult::error("Merchant does not exist".to_string());
        };
        let mut total_price = NotNan::new(0.0).unwrap();
        let mut order_items: Vec<OrderItem> = Vec::new();
        for item in items {
            if item.quantity == 0 {
                return ExecutionResult::error(format!("Invalid quantity {} for product {}", item.quantity, item.product));
            }
            // Find the product price
            let Some(product) = merchant.menu.iter().find(|p| p.product == item.product) else {
                return ExecutionResult::error(format!("Product {} does not exist in {}'s menu", item.product, merchant_name));
            };
            total_price += NotNan::new(product.price.into_inner() * item.quantity as f64).unwrap();
            order_items.push(OrderItem {
                product: item.product,
                quantity: item.quantity,
                price_per_unit: product.price,
            });
        }
        // Check if the balance is sufficient
        let user = self.users.get_mut(&username).unwrap();
        if total_price > user.balance {
            return ExecutionResult::error("Insufficient balance to place the order".to_string());
        }
        // Deduct the balance and create the order
        user.balance -= total_price;
        let order = FoodOrder {
            user_name: username.to_string(),
            merchant_name: merchant_name.to_string(),
            items: order_items,
            total_price,
        };
        self.orders.push(order);
        ExecutionResult::success(format!(
            "Food delivery order successfully placed with {}. Total amount: {} yuan",
            merchant_name, total_price
        ))    
    }
    // the output format is slightly different from the original implementation for convenience
    pub fn get_products(&self, merchant_name: String) -> ExecutionResult {
        let Some(merchant) = self.merchant_list.get(&merchant_name) else {
            return ExecutionResult::error(format!("Merchant '{}' does not exist", merchant_name));
        };
        let products_str = serde_json::to_string(&merchant.menu).unwrap();
        ExecutionResult::success(format!("Products for {}: {}", merchant_name, products_str))
    }

    pub fn view_orders(&self, user_name: String) -> ExecutionResult {
        let user_orders: Vec<&FoodOrder> = self
            .orders
            .iter()
            .filter(|order| order.user_name == user_name)
            .collect();
        if user_orders.is_empty() {
            return ExecutionResult::error(format!("User {} has no order records", user_name));
        }
        let orders_str = serde_json::to_string(&user_orders).unwrap();
        ExecutionResult::success(format!("Orders for {}: {}", user_name, orders_str))
    }

    pub fn search_orders(&self, keyword: String) -> ExecutionResult {
        let matched_orders: Vec<&FoodOrder> = self
            .orders
            .iter()
            .filter(|order| {
                order.merchant_name.to_lowercase().contains(&keyword.to_lowercase())
                    || order.items.iter().any(|item| item.product.to_lowercase().contains(&keyword.to_lowercase()))
            })
            .collect();
        if matched_orders.is_empty() {
            return ExecutionResult::error("No matching orders found".to_string());
        }
        let orders_str = serde_json::to_string(&matched_orders).unwrap();
        ExecutionResult::success(format!("Matched orders for keyword '{}': {}", keyword, orders_str))
    }
}
