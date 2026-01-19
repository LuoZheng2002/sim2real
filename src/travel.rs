use core::panic;
use std::result;

use indexmap::IndexMap;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::base_api::ExecutionResult;

/// Travel system user
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TravelUser {
    pub user_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub cash_balance: NotNan<f64>,
    pub bank_balance: NotNan<f64>,
    pub membership_level: String, // "regular", "silver", "gold"
}

/// Flight information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Flight {
    pub flight_no: String,
    pub origin: String,
    pub destination: String,
    pub depart_time: String, // format: "YYYY-MM-DD HH:MM:SS"
    pub arrival_time: String,
    pub status: String, // "available"
    pub seats_available: u32,
    pub economy_price: u32,
    pub business_price: u32,
}

/// Flight reservation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reservation {
    pub reservation_id: String,
    pub user_id: String,
    pub flight_no: String,
    // shows in get_reservation_details
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flight_info: Option<Flight>,
    pub payment_method: String, // "cash" or "bank"
    pub cabin: String,          // "Economy Class" or "Business Class"
    pub baggage: u32,
    pub origin: String,
    pub destination: String,
}

/// Travel API state (does NOT inherit from BaseApi)
/// Python: scenariosen/travel.py
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Travel {
    pub users: IndexMap<String, TravelUser>, // key: user_id (e.g., "user1")
    pub flights: Vec<Flight>,
    pub reservations: Vec<Reservation>,
}

impl Default for Travel {
    fn default() -> Self {
        let users: IndexMap<String, TravelUser> = vec![
            (
                "user1".to_string(),
                TravelUser {
                    user_name: "Eve".to_string(),
                    password: Some("password123".to_string()),
                    cash_balance: NotNan::new(2000.0).unwrap(),
                    bank_balance: NotNan::new(50000.0).unwrap(),
                    membership_level: "regular".to_string(),
                },
            ),
            (
                "user2".to_string(),
                TravelUser {
                    user_name: "Frank".to_string(),
                    password: Some("password456".to_string()),
                    cash_balance: NotNan::new(8000.0).unwrap(),
                    bank_balance: NotNan::new(8000.0).unwrap(),
                    membership_level: "silver".to_string(),
                },
            ),
            (
                "user3".to_string(),
                TravelUser {
                    user_name: "Grace".to_string(),
                    password: Some("password789".to_string()),
                    cash_balance: NotNan::new(1000.0).unwrap(),
                    bank_balance: NotNan::new(5000.0).unwrap(),
                    membership_level: "gold".to_string(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let flights: Vec<Flight> = vec![
            Flight {
                flight_no: "CA1234".to_string(),
                origin: "Beijing".to_string(),
                destination: "Shanghai".to_string(),
                depart_time: "2024-07-15 08:00:00".to_string(),
                arrival_time: "2024-07-15 10:30:00".to_string(),
                status: "available".to_string(),
                seats_available: 5,
                economy_price: 1200,
                business_price: 3000,
            },
            Flight {
                flight_no: "MU5678".to_string(),
                origin: "Shanghai".to_string(),
                destination: "Beijing".to_string(),
                depart_time: "2024-07-16 09:00:00".to_string(),
                arrival_time: "2024-07-16 11:30:00".to_string(),
                status: "available".to_string(),
                seats_available: 3,
                economy_price: 1900,
                business_price: 3000,
            },
            Flight {
                flight_no: "CZ4321".to_string(),
                origin: "Shanghai".to_string(),
                destination: "Beijing".to_string(),
                depart_time: "2024-07-16 20:00:00".to_string(),
                arrival_time: "2024-07-16 22:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 2500,
                business_price: 4000,
            },
            Flight {
                flight_no: "CZ4352".to_string(),
                origin: "Shanghai".to_string(),
                destination: "Beijing".to_string(),
                depart_time: "2024-07-17 20:00:00".to_string(),
                arrival_time: "2024-07-17 22:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1600,
                business_price: 2500,
            },
            Flight {
                flight_no: "MU3561".to_string(),
                origin: "Beijing".to_string(),
                destination: "Nanjing".to_string(),
                depart_time: "2024-07-18 08:00:00".to_string(),
                arrival_time: "2024-07-18 10:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 4000,
            },
            Flight {
                flight_no: "MU1566".to_string(),
                origin: "Beijing".to_string(),
                destination: "Nanjing".to_string(),
                depart_time: "2024-07-18 20:00:00".to_string(),
                arrival_time: "2024-07-18 22:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 4000,
            },
            Flight {
                flight_no: "CZ1765".to_string(),
                origin: "Nanjing".to_string(),
                destination: "Shenzhen".to_string(),
                depart_time: "2024-07-17 20:30:00".to_string(),
                arrival_time: "2024-07-17 22:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 2500,
            },
            Flight {
                flight_no: "CZ1765".to_string(),
                origin: "Nanjing".to_string(),
                destination: "Shenzhen".to_string(),
                depart_time: "2024-07-18 12:30:00".to_string(),
                arrival_time: "2024-07-18 15:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 2500,
            },
            Flight {
                flight_no: "MH1765".to_string(),
                origin: "Xiamen".to_string(),
                destination: "Chengdu".to_string(),
                depart_time: "2024-07-17 12:30:00".to_string(),
                arrival_time: "2024-07-17 15:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 2500,
            },
            Flight {
                flight_no: "MH2616".to_string(),
                origin: "Chengdu".to_string(),
                destination: "Xiamen".to_string(),
                depart_time: "2024-07-18 18:30:00".to_string(),
                arrival_time: "2024-07-18 21:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 2500,
            },
            Flight {
                flight_no: "MH2616".to_string(),
                origin: "Chengdu".to_string(),
                destination: "Fuzhou".to_string(),
                depart_time: "2024-07-16 18:30:00".to_string(),
                arrival_time: "2024-07-16 21:00:00".to_string(),
                status: "available".to_string(),
                seats_available: 8,
                economy_price: 1500,
                business_price: 2500,
            },
        ];
        let reservations = vec![
            Reservation {
                reservation_id: "res_1".to_string(),
                user_id: "user1".to_string(),
                flight_no: "CA1234".to_string(),
                flight_info: None,
                payment_method: "bank".to_string(),
                cabin: "Economy Class".to_string(),
                baggage: 1,
                origin: "Beijing".to_string(),
                destination: "Shanghai".to_string(),
            },
            Reservation {
                reservation_id: "res_2".to_string(),
                user_id: "user1".to_string(),
                flight_no: "MU5678".to_string(),
                flight_info: None,
                payment_method: "bank".to_string(),
                cabin: "Business Class".to_string(),
                baggage: 1,
                origin: "Shanghai".to_string(),
                destination: "Beijing".to_string(),
            },
            Reservation {
                reservation_id: "res_3".to_string(),
                user_id: "user2".to_string(),
                flight_no: "MH1765".to_string(),
                flight_info: None,
                payment_method: "bank".to_string(),
                cabin: "Business Class".to_string(),
                baggage: 1,
                origin: "Xiamen".to_string(),
                destination: "Chengdu".to_string(),
            },
            Reservation {
                reservation_id: "res_4".to_string(),
                user_id: "user2".to_string(),
                flight_no: "MU2616".to_string(),
                flight_info: None,
                payment_method: "bank".to_string(),
                cabin: "Business Class".to_string(),
                baggage: 1,
                origin: "Chengdu".to_string(),
                destination: "Xiamen".to_string(),
            },
        ];
        Travel {
            users,
            flights,
            reservations,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct GetFlightDetailsArgs {
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub destination: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct GetUserDetailsArgs {
    pub user_id: String,
    pub password: String,
}
#[derive(Clone, Deserialize)]
pub struct GetReservationDetailsArgs {
    #[serde(default)]
    pub reservation_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct FindTransferFlightsArgs {
    pub origin_city: String,
    pub transfer_city: String,
    pub destination_city: String,
}
#[derive(Clone, Deserialize)]
pub struct ReserveFlightArgs {
    pub user_id: String,
    pub password: String,
    pub flight_no: String,
    pub cabin: String,
    pub payment_method: String,
    pub baggage_count: usize,
}

#[derive(Clone, Deserialize)]
pub struct ModifyFlightArgs {
    pub user_id: String,
    pub reservation_id: String,
    #[serde(default)]
    pub new_flight_no: Option<String>,
    #[serde(default)]
    pub new_cabin: Option<String>,
    #[serde(default)]
    pub add_baggage: Option<usize>,
    #[serde(default)]
    pub new_payment_method: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct CancelReservationArgs {
    pub user_id: String,
    pub reservation_id: String,
    pub reason: String,
}

impl Travel {
    pub fn get_flight_details(
        &self,
        origin: Option<String>,
        destination: Option<String>,
    ) -> ExecutionResult {
        let mut flights = self.flights.clone();
        if let Some(orig) = origin {
            flights = flights
                .into_iter()
                .filter(|flight| flight.origin == orig)
                .collect();
        }
        if let Some(dest) = destination {
            flights = flights
                .into_iter()
                .filter(|flight| flight.destination == dest)
                .collect();
        }
        if flights.is_empty() {
            return ExecutionResult::error(
                "There are no direct flights that meet the criteria.".to_string(),
            );
        }
        let flights_str = serde_json::to_string(&flights).unwrap();
        ExecutionResult::success(format!("Flight details: {}", flights_str))
    }
    pub fn get_user_details(&self, user_id: String, password: String) -> ExecutionResult {
        if let Some(user) = self.users.get(&user_id)
            && user.password == Some(password.to_string())
        {
            let mut user_info = user.clone();
            user_info.password = None; // Do not expose password
            let user_info_str = serde_json::to_string(&user_info).unwrap();
            ExecutionResult::success(format!("User details: {}", user_info_str))
        } else {
            return ExecutionResult::error("Incorrect username or password.".to_string());
        }
    }
    pub fn get_reservation_details(
        &self,
        reservation_id: Option<String>,
        user_id: Option<String>,
    ) -> ExecutionResult {
        let mut reservations = self.reservations.clone();
        if let Some(res_id) = reservation_id {
            reservations = reservations
                .into_iter()
                .filter(|res| res.reservation_id == res_id)
                .collect();
        } else if let Some(u_id) = user_id {
            reservations = reservations
                .into_iter()
                .filter(|res| res.user_id == u_id)
                .collect();
        } else {
            return ExecutionResult::error(
                "Please provide a valid reservation ID or user ID".to_string(),
            );
        }
        // inject detailed flight info
        let detailed_reservations = reservations
            .into_iter()
            .map(|mut res| {
                // only inject if can be found in self.flights
                if let Some(flight) = self
                    .flights
                    .iter()
                    .find(|flight| flight.flight_no == res.flight_no)
                {
                    res.flight_info = Some(flight.clone());
                }
                res
            })
            .collect::<Vec<Reservation>>();
        let detailed_reservations_str = serde_json::to_string(&detailed_reservations).unwrap();
        ExecutionResult::success(format!(
            "Reservation details: {}",
            detailed_reservations_str
        ))
    }
    // helper function, not directly invoked
    pub fn authenticate_user(&self, user_id: &str, password: &str) -> bool {
        if let Some(user) = self.users.get(user_id) {
            if user.password == Some(password.to_string()) {
                return true;
            }
        }
        false
    }
    // helper function, not directly invoked
    fn get_baggage_allowance(membership_level: &str, cabin_class: &str) -> usize {
        match (membership_level, cabin_class) {
            ("regular", "Economy Class") => 1,
            ("regular", "Business Class") => 2,
            ("silver", "Economy Class") => 2,
            ("silver", "Business Class") => 3,
            ("gold", "Economy Class") => 3,
            ("gold", "Business Class") => 3,
            _ => panic!("Unknown membership level or cabin class"),
        }
    }
    pub fn find_transfer_flights(
        &self,
        origin_city: String,
        transfer_city: String,
        destination_city: String,
    ) -> ExecutionResult {
        // Get flights from departure city to transfer city
        let first_leg_flights: Vec<&Flight> = self
            .flights
            .iter()
            .filter(|flight| {
                flight.origin == origin_city
                    && flight.destination == transfer_city
                    && flight.status == "available"
            })
            .collect();

        // Get flights from transfer city to destination city
        let second_leg_flights: Vec<&Flight> = self
            .flights
            .iter()
            .filter(|flight| {
                flight.origin == transfer_city
                    && flight.destination == destination_city
                    && flight.status == "available"
            })
            .collect();

        // Combine first and second leg flights into connecting flights
        // rename it from transfer_flights to connecting_flights, however, the function name remains the same for compatibility
        let mut connecting_flights = Vec::new();
        for first_leg in &first_leg_flights {
            for second_leg in &second_leg_flights {
                // Here we should ideally check the timing constraints (arrival time of first < depart time of second)
                // but for simplicity, we skip that check in this implementation.
                connecting_flights.push(((*first_leg).clone(), (*second_leg).clone()));
            }
        }

        if connecting_flights.is_empty() {
            return ExecutionResult::error(
                "No connecting flights found that meet the criteria.".to_string(),
            );
        }

        let connecting_flights_str = serde_json::to_string(&connecting_flights).unwrap();
        ExecutionResult::success(format!("Connecting flights: {}", connecting_flights_str))
    }
    // helper function, not directly invoked
    pub fn calculate_baggage_fee(
        membership_level: &str,
        cabin_class: &str,
        baggage_count: usize,
    ) -> usize {
        let allowance = Self::get_baggage_allowance(membership_level, cabin_class);
        let additional_baggage = baggage_count.saturating_sub(allowance);
        additional_baggage * 50 // assuming each additional baggage costs 50
    }
    // helper function, not directly invoked
    pub fn update_balance(travel_user: &mut TravelUser, payment_method: &str, amount: f64) -> bool {
        let amount = NotNan::new(amount).unwrap();
        match payment_method {
            "cash" => {
                if travel_user.cash_balance < amount {
                    return false;
                }
                travel_user.cash_balance =
                    NotNan::new(travel_user.cash_balance.into_inner() + amount.into_inner())
                        .unwrap();
            }
            "bank" => {
                if travel_user.bank_balance < amount {
                    return false;
                }
                travel_user.bank_balance =
                    NotNan::new(travel_user.bank_balance.into_inner() + amount.into_inner())
                        .unwrap();
            }
            _ => panic!("Unknown payment method"),
        }
        true
    }
    pub fn reserve_flight(
        &mut self,
        user_id: String,
        password: String,
        flight_no: String,
        cabin: String,
        payment_method: String,
        baggage_count: usize,
    ) -> ExecutionResult {
        let mut flights = std::mem::take(&mut self.flights);
        let mut travel_users = std::mem::take(&mut self.users);
        if !self.authenticate_user(&user_id, &password) {
            return ExecutionResult::error(
                "Authentication failed. Incorrect username or password.".to_string(),
            );
        };
        let Some(flight) = flights.iter_mut().find(|f| f.flight_no == flight_no) else {
            return ExecutionResult::error(format!("Flight {} not found.", flight_no));
        };
        if flight.status != "available" || flight.seats_available == 0 {
            return ExecutionResult::error(format!(
                "Flight {} is not available for booking or has no seats available.",
                flight_no
            ));
        }
        let price = match cabin.as_str() {
            "Economy Class" => flight.economy_price,
            "Business Class" => flight.business_price,
            _ => {
                panic!("Unknown cabin class");
            }
        };
        let mut total_cost: f64 = price as f64;
        let user = travel_users.get_mut(&user_id).unwrap();
        let baggage_fee =
            Self::calculate_baggage_fee(&user.membership_level, &cabin, baggage_count);
        total_cost += baggage_fee as f64;
        if !Self::update_balance(user, &payment_method, -total_cost) {
            return ExecutionResult::error(format!(
                "Your {} balance is insufficient. Please consider using another payment method.",
                payment_method
            ));
        }
        flight.seats_available -= 1;
        let reservation_id = format!("res_{}", self.reservations.len() + 1);
        let reservation = Reservation {
            reservation_id: reservation_id.clone(),
            user_id: user_id.to_string(),
            flight_no: flight_no.to_string(),
            flight_info: None,
            payment_method: payment_method.to_string(),
            cabin: cabin.to_string(),
            baggage: baggage_count as u32,
            origin: flight.origin.clone(),
            destination: flight.destination.clone(),
        };
        self.reservations.push(reservation);

        // return the flights back
        self.flights = flights;
        self.users = travel_users;
        ExecutionResult::success(format!(
            "Booking successful. Reservation ID: {}. Total cost: {} yuan (including baggage fees).",
            reservation_id, total_cost
        ))
    }
    // helper function, not directly invoked
    fn calculate_price_difference(flight: &Flight, old_cabin: &str, new_cabin: &str) -> f64 {
        let old_price = match old_cabin {
            "Economy Class" => flight.economy_price,
            "Business Class" => flight.business_price,
            _ => panic!("Unknown cabin class"),
        };
        let new_price = match new_cabin {
            "Economy Class" => flight.economy_price,
            "Business Class" => flight.business_price,
            _ => panic!("Unknown cabin class"),
        };
        (new_price as f64) - (old_price as f64)
    }

    pub fn modify_flight(
        &mut self,
        user_id: String,
        reservation_id: String,
        new_flight_no: Option<String>,
        new_cabin: Option<String>,
        add_baggage: Option<usize>,
        new_payment_method: Option<String>,
    ) -> ExecutionResult {
        let mut reservations = std::mem::take(&mut self.reservations);
        // the following is intentionally left to have a warning of "not need to be mutable", because it is likely we need to
        // modify flight information in the future (seat availability when changing flight_no)
        let mut flights = std::mem::take(&mut self.flights);
        let mut travel_users = std::mem::take(&mut self.users);
        let Some(reservation) = reservations
            .iter_mut()
            .find(|res| res.reservation_id == reservation_id && res.user_id == user_id)
        else {
            return ExecutionResult::error("Reservation not found for the given user.".to_string());
        };

        let Some(current_flight) = flights
            .iter()
            .find(|f| f.flight_no == reservation.flight_no)
        else {
            return ExecutionResult::error("Current flight information not found.".to_string());
        };
        let payment_method = new_payment_method.unwrap_or(reservation.payment_method.clone());
        let Some(user) = travel_users.get_mut(&user_id) else {
            return ExecutionResult::error("User information not found.".to_string());
        };

        let mut result_messages: Vec<String> = Vec::new();
        if let Some(new_flight_no) = new_flight_no
            && new_flight_no != reservation.flight_no
        {
            let Some(new_flight) = flights.iter().find(|f| f.flight_no == new_flight_no) else {
                return ExecutionResult::error(
                    "Flight change failed: Invalid new flight number.".to_string(),
                );
            };
            if new_flight.origin == current_flight.origin
                && new_flight.destination == current_flight.destination
            {
                // this is the logic in the original python code, which only changes the reservation record but not flight seat availability
                // this might be logically wrong, but we keep it for compatibility
                reservation.flight_no = new_flight_no.to_string();
                result_messages.push("Flight number has been changed.".to_string());
            } else {
                return ExecutionResult::error(
                    "Flight change failed: Destination does not match.".to_string(),
                );
            }
        }
        if let Some(new_cabin) = new_cabin
            && new_cabin != reservation.cabin
        {
            let price_difference =
                Self::calculate_price_difference(current_flight, &reservation.cabin, &new_cabin);
            let paid_or_refunded = match price_difference >= 0.0 {
                true => "paid",
                false => "refunded",
            };
            if Self::update_balance(user, &payment_method, -price_difference) {
                result_messages.push(format!(
                    "Cabin change successful. Price difference {}: {}.",
                    paid_or_refunded,
                    price_difference.abs()
                ));
                reservation.cabin = new_cabin.to_string();
            } else {
                result_messages
                    .push("Insufficient balance to pay the cabin price difference.".to_string());
            }
        }
        if let Some(add_baggage) = add_baggage
            && add_baggage > 0
        {
            let total_baggage = reservation.baggage as usize + add_baggage;
            let new_baggage_cost = Self::calculate_baggage_fee(
                &user.membership_level,
                &reservation.cabin,
                total_baggage,
            );
            let old_baggage_cost = Self::calculate_baggage_fee(
                &user.membership_level,
                &reservation.cabin,
                reservation.baggage as usize,
            );
            let baggage_cost: f64 = new_baggage_cost as f64 - old_baggage_cost as f64;
            if Self::update_balance(user, &payment_method, -(baggage_cost as f64)) {
                if baggage_cost > 0.0 {
                    result_messages.push(format!(
                        "Baggage has been added. Additional fee to be paid: {}.",
                        baggage_cost
                    ));
                } else {
                    result_messages.push("Baggage has been added. No additional fee.".to_string());
                }
                reservation.baggage = total_baggage as u32;
            } else {
                result_messages
                    .push("Insufficient balance to pay the additional baggage fees.".to_string());
            }
        }
        if result_messages.is_empty() {
            result_messages.push("Modification completed with no additional fees.".to_string());
        }
        // put reservations and flights back
        self.reservations = reservations;
        self.flights = flights;
        self.users = travel_users;
        let final_message = result_messages.join(" ");
        ExecutionResult::success(final_message)
    }

    pub fn cancel_reservation(
        &mut self,
        user_id: String,
        reservation_id: String,
        reason: String,
    ) -> ExecutionResult {
        // Set the default current time to July 14, 2024, 6:00 AM
        let current_time =
            chrono::NaiveDateTime::parse_from_str("2024-07-14 06:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        let mut reservations = std::mem::take(&mut self.reservations);
        let mut flights = std::mem::take(&mut self.flights);
        let mut travel_users = std::mem::take(&mut self.users);

        let Some(user) = travel_users.get_mut(&user_id) else {
            return ExecutionResult::error("Invalid user ID.".to_string());
        };
        let Some(reservation) = reservations
            .iter()
            .find(|r| r.reservation_id == reservation_id && r.user_id == user_id)
        else {
            return ExecutionResult::error(
                "Invalid reservation ID or it does not belong to the user.".to_string(),
            );
        };
        let Some(flight) = flights
            .iter()
            .find(|f| f.flight_no == reservation.flight_no)
        else {
            return ExecutionResult::error("Invalid flight information.".to_string());
        };
        let depart_time =
            chrono::NaiveDateTime::parse_from_str(&flight.depart_time, "%Y-%m-%d %H:%M:%S")
                .unwrap();
        if current_time > depart_time {
            return ExecutionResult::error(
                "The flight segment has been used and cannot be canceled.".to_string(),
            );
        }
        let time_until_departure = depart_time - current_time;

        // Get the flight price
        let flight_price = match reservation.cabin.as_str() {
            "Economy Class" => flight.economy_price as f64,
            "Business Class" => flight.business_price as f64,
            _ => panic!("Unknown cabin class"),
        };
        // Cancellation policy and refund calculation
        let execution_result = if reason == "The airline has canceled the flight." {
            // Airline cancels the flight, full refund
            let refund_amount = flight_price;
            assert!(refund_amount >= 0.0);
            // the original process_refund function in python adds the amount to user's cash balance
            Self::update_balance(user, "cash", refund_amount);
            ExecutionResult::success(format!(
                "The flight has been canceled. Your reservation will be canceled free of charge, and {} yuan has been refunded.",
                refund_amount
            ))
        } else if time_until_departure > chrono::Duration::hours(24) {
            // More than 24 hours before departure, free cancellation
            let refund_amount = flight_price;
            assert!(refund_amount >= 0.0);
            Self::update_balance(user, "cash", refund_amount);
            ExecutionResult::success(format!(
                "More than 24 hours before departure. Free cancellation successful, {} yuan has been refunded.",
                refund_amount
            ))
        } else {
            // If not eligible for free cancellation, set a cancellation fee as needed
            let cancel_fee = flight_price * 0.1; // Assume a cancellation fee of 10% of the ticket price
            let refund_amount = flight_price - cancel_fee;
            assert!(refund_amount >= 0.0);
            Self::update_balance(user, "cash", refund_amount);
            ExecutionResult::success(format!(
                "Less than 24 hours before departure. A cancellation fee of {} yuan has been deducted, and {} yuan has been refunded.",
                cancel_fee, refund_amount
            ))
        };
        // the following does not appear in the original python code, which might be a bug
        // commonly, we need to remove the reservation record and increase the available seats after cancellation
        // Increase the available seats on the flight
        if let Some(flight) = flights
            .iter_mut()
            .find(|f| f.flight_no == reservation.flight_no)
        {
            flight.seats_available += 1;
        }
        // Remove the reservation
        reservations.retain(|r| r.reservation_id != reservation_id);
        // put reservations and flights back
        self.reservations = reservations;
        self.flights = flights;
        self.users = travel_users;
        execution_result
    }
}
