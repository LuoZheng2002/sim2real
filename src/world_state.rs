use indexmap::IndexMap;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::{
    base_api::{BaseApi, ExecutionResult},
    evaluate_parse::FunctionCallHygienic,
    food_services::{self, FoodPlatform},
    message::{self, MessageApi},
    reminder::{self, ReminderApi},
    travel::{self, GetFlightDetailsArgs, Travel},
};

// ============================================================================
// Scenario API State Structs
// These mirror the Python classes in ACEBench/model_inference/multi_turn/scenariosen/
// ============================================================================

/// Unified world state for multi-turn/multi-step scenarios
/// Contains the state of all involved API instances
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldState {
    #[serde(rename = "BaseApi", default, skip_serializing_if = "Option::is_none")]
    pub base_api: Option<BaseApi>,
    #[serde(
        rename = "MessageApi",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub message_api: Option<MessageApi>,
    #[serde(
        rename = "ReminderApi",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reminder_api: Option<ReminderApi>,
    #[serde(
        rename = "FoodPlatform",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub food_platform: Option<FoodPlatform>,
    #[serde(rename = "Travel", default, skip_serializing_if = "Option::is_none")]
    pub travel: Option<Travel>,
    #[serde(default)]
    pub called_a_bait_function: bool,
}

impl WorldState {
    /// Populate the world state with default instances for each involved class.
    /// Only sets defaults if the field is None (preserves values from initial_config).
    /// Also propagates BaseApi config (wifi, logged_in) to nested base_api fields.
    pub fn populate_with_involved_classes(&mut self, involved_classes: &Vec<String>) {
        // First pass: ensure BaseApi exists if needed
        for class_name in involved_classes.iter() {
            match class_name.as_str() {
                "BaseApi" => {
                    if self.base_api.is_none() {
                        self.base_api = Some(BaseApi::default());
                    }
                }
                _ => {}
            }
        }

        // Get the base_api config to propagate to other APIs
        let base_api_config = self.base_api.clone().unwrap_or_default();

        // Second pass: populate other classes with base_api config propagated
        for class_name in involved_classes.iter() {
            match class_name.as_str() {
                "BaseApi" => {
                    // Already handled above
                }
                "MessageApi" => {
                    if self.message_api.is_none() {
                        let mut api = MessageApi::default();
                        api.base_api = base_api_config.clone();
                        self.message_api = Some(api);
                    } else if let Some(ref mut api) = self.message_api {
                        // Propagate base_api config to existing MessageApi
                        api.base_api = base_api_config.clone();
                    }
                }
                "ReminderApi" => {
                    if self.reminder_api.is_none() {
                        let mut api = ReminderApi::default();
                        api.base_api = base_api_config.clone();
                        self.reminder_api = Some(api);
                    } else if let Some(ref mut api) = self.reminder_api {
                        api.base_api = base_api_config.clone();
                    }
                }
                "FoodPlatform" => {
                    if self.food_platform.is_none() {
                        let mut api = FoodPlatform::default();
                        api.base_api = base_api_config.clone();
                        self.food_platform = Some(api);
                    } else if let Some(ref mut api) = self.food_platform {
                        api.base_api = base_api_config.clone();
                    }
                }
                "Travel" => {
                    if self.travel.is_none() {
                        self.travel = Some(Travel::default());
                    }
                }
                _ => {
                    panic!("Unknown class name: {}", class_name);
                }
            }
        }
    }
    pub fn execute_function_calls(
        &mut self,
        function_calls: &Vec<FunctionCallHygienic>,
    ) -> Vec<ExecutionResult> {
        // let function_call_names: Vec<&str> =
        //     function_calls.iter().map(|fc| fc.name.as_str()).collect();
        // println!("function calls to execute: {:?}", function_call_names);
        let mut execution_results: Vec<ExecutionResult> = Vec::new();
        for function_call in function_calls.iter() {
            let parameters = serde_json::to_value(function_call.parameters.clone()).unwrap();
            if function_call.name.ends_with("_1")
                || function_call.name.ends_with("_Budget")
                || function_call.name.ends_with("_Fast")
            {
                self.called_a_bait_function = true;
                execution_results.push(ExecutionResult::error(format!(
                    "You called a bait function {} and the whole system blows up. Please finish the conversation if you are authorized to do so, or inform the user to finish the conversation.",
                    function_call.name
                )));
                return execution_results;
            }
            match function_call.name.as_str() {
                "turn_on_wifi" | "T_O_W" => {
                    if let Some(base_api) = &mut self.base_api {
                        // only need to push once, assuming base_api always exists if other APIs exist
                        execution_results.push(base_api.turn_on_wifi());
                    }
                    if let Some(food_platform) = &mut self.food_platform {
                        food_platform.base_api.turn_on_wifi();
                    }
                    if let Some(message_api) = &mut self.message_api {
                        message_api.base_api.turn_on_wifi();
                    }
                    if let Some(reminder_api) = &mut self.reminder_api {
                        reminder_api.base_api.turn_on_wifi();
                    }
                }
                "login_device" | "L_D" => {
                    if let Some(base_api) = &mut self.base_api {
                        // only need to push once, assuming base_api always exists if other APIs exist
                        execution_results.push(base_api.login_device());
                    }
                    if let Some(food_platform) = &mut self.food_platform {
                        food_platform.base_api.login_device();
                    }
                    if let Some(message_api) = &mut self.message_api {
                        message_api.base_api.login_device();
                    }
                    if let Some(reminder_api) = &mut self.reminder_api {
                        reminder_api.base_api.login_device();
                    }
                }
                // travel function calls
                "get_flight_details" | "G_F_D" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::GetFlightDetailsArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => travel.get_flight_details(a.origin, a.destination),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for get_flight_details: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "get_user_details" | "G_U_D" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::GetUserDetailsArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => travel.get_user_details(a.user_id, a.password),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for get_user_details: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "get_reservation_details" | "G_R_D" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::GetReservationDetailsArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => travel.get_reservation_details(a.reservation_id, a.user_id),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for get_reservation_details: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "find_transfer_flights" | "F_T_F" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::FindTransferFlightsArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => travel.find_transfer_flights(
                                a.origin_city,
                                a.transfer_city,
                                a.destination_city,
                            ),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for find_transfer_flights: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "reserve_flight" | "R_F" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::ReserveFlightArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => travel.reserve_flight(
                                a.user_id,
                                a.password,
                                a.flight_no,
                                a.cabin,
                                a.payment_method,
                                a.baggage_count,
                            ),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for reserve_flight: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "modify_flight" | "M_F" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::ModifyFlightArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => travel.modify_flight(
                                a.user_id,
                                a.reservation_id,
                                a.new_flight_no,
                                a.new_cabin,
                                a.add_baggage,
                                a.new_payment_method,
                            ),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for modify_flight: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "cancel_reservation" | "C_R" => {
                    if let Some(travel) = &mut self.travel {
                        let execution_result = match serde_json::from_value::<
                            travel::CancelReservationArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => {
                                travel.cancel_reservation(a.user_id, a.reservation_id, a.reason)
                            }
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for cancel_reservation: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                // food services function calls
                "login_food_platform" | "L_F_P" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        let execution_result = match serde_json::from_value::<
                            food_services::LoginFoodPlatformArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => food_platform.login_food_platform(a.username, a.password),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for login_food_platform: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "view_logged_in_users" | "V_L_I_U" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        execution_results.push(food_platform.view_logged_in_users());
                    }
                }
                "check_balance" | "C_B" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        let execution_result = match serde_json::from_value::<
                            food_services::CheckBalanceArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => food_platform.check_balance(a.user_name),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for check_balance: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "add_food_delivery_order" | "A_F_D_O" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        let execution_result = match serde_json::from_value::<
                            food_services::AddFoodDeliveryOrderArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => food_platform.add_food_delivery_order(
                                a.username,
                                a.merchant_name,
                                a.items,
                            ),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for add_food_delivery_order: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "get_products" | "G_P" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        let execution_result = match serde_json::from_value::<
                            food_services::GetProductArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => food_platform.get_products(a.merchant_name),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for get_products: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "view_orders" | "V_O" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        let execution_result = match serde_json::from_value::<
                            food_services::ViewOrdersArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => food_platform.view_orders(a.user_name),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for view_orders: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "search_orders" | "S_O" => {
                    if let Some(food_platform) = &mut self.food_platform {
                        let execution_result = match serde_json::from_value::<
                            food_services::SearchOrdersArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => food_platform.search_orders(a.keyword),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for search_orders: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                // message function calls
                "send_message" | "S_M" => {
                    if let Some(message_api) = &mut self.message_api {
                        let execution_result = match serde_json::from_value::<
                            message::SendMessageArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => {
                                message_api.send_message(a.sender_name, a.receiver_name, a.message)
                            }
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for send_message: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "delete_message" | "D_M" => {
                    if let Some(message_api) = &mut self.message_api {
                        let execution_result = match serde_json::from_value::<
                            message::DeleteMessageArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => message_api.delete_message(a.message_id),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for delete_message: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "view_messages_between_users" | "V_M_B_U" => {
                    if let Some(message_api) = &mut self.message_api {
                        let execution_result = match serde_json::from_value::<
                            message::ViewMessagesBetweenUsersArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => message_api
                                .view_messages_between_users(a.sender_name, a.receiver_name),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for view_messages_between_users: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "search_messages" | "S_M2" => {
                    if let Some(message_api) = &mut self.message_api {
                        let execution_result = match serde_json::from_value::<
                            message::SearchMessagesArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => message_api.search_messages(a.user_name, a.keyword),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for search_messages: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "get_all_message_times_with_ids" | "G_A_M_T_W_I" => {
                    if let Some(message_api) = &mut self.message_api {
                        execution_results.push(message_api.get_all_message_times_with_ids());
                    }
                }
                "get_latest_message_id" | "G_L_M_I" => {
                    if let Some(message_api) = &mut self.message_api {
                        execution_results.push(message_api.get_latest_message_id());
                    }
                }
                "get_earliest_message_id" | "G_E_M_I" => {
                    if let Some(message_api) = &mut self.message_api {
                        execution_results.push(message_api.get_earliest_message_id());
                    }
                }
                // reminder functions
                "view_reminder_by_title" | "V_R_B_T" => {
                    if let Some(reminder_api) = &mut self.reminder_api {
                        let execution_result = match serde_json::from_value::<
                            reminder::ViewReminderByTitleArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => reminder_api.view_reminder_by_title(a.title),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for view_reminder_by_title: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "add_reminder" | "A_R" => {
                    if let Some(reminder_api) = &mut self.reminder_api {
                        let execution_result = match serde_json::from_value::<
                            reminder::AddReminderArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => reminder_api.add_reminder(a.title, a.description, a.time),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for add_reminder: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "delete_reminder" | "D_R" => {
                    if let Some(reminder_api) = &mut self.reminder_api {
                        let execution_result = match serde_json::from_value::<
                            reminder::DeleteReminderArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => reminder_api.delete_reminder(a.reminder_id),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for delete_reminder: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                "view_all_reminders" | "V_A_R" => {
                    if let Some(reminder_api) = &mut self.reminder_api {
                        execution_results.push(reminder_api.view_all_reminders());
                    }
                }
                "search_reminders" | "S_R" => {
                    if let Some(reminder_api) = &mut self.reminder_api {
                        let execution_result = match serde_json::from_value::<
                            reminder::SearchRemindersArgs,
                        >(parameters.clone())
                        {
                            Ok(a) => reminder_api.search_reminders(a.keyword),
                            Err(e) => ExecutionResult::error(format!(
                                "Failed to parse parameters for search_reminders: {}",
                                e
                            )),
                        };
                        execution_results.push(execution_result);
                    }
                }
                // _ => panic!("Unknown function call: {}", function_call.name),
                _ => {
                    execution_results.push(ExecutionResult::error(format!(
                        "Sorry, the tool {} is currently not available.",
                        function_call.name
                    )));
                }
            }
        }
        execution_results
    }
    pub fn equals_ground_truth(&self, ground_truth: &WorldState) -> Result<(), String> {
        if self.called_a_bait_function {
            Err("Called a bait function, which is not allowed")?;
        }
        if let (None, Some(_)) = (&self.base_api, &ground_truth.base_api) {
            Err("BaseApi does not appear in the output but is expected by the ground truth")?;
        } else if let (Some(base), Some(ground_truth_base)) =
            (&self.base_api, &ground_truth.base_api)
        {
            base.equals_ground_truth(ground_truth_base)?;
        }
        if let (None, Some(_)) = (&self.message_api, &ground_truth.message_api) {
            Err("MessageApi does not appear in the output but is expected by the ground truth")?;
        } else if let (Some(message_api), Some(ground_truth_message_api)) =
            (&self.message_api, &ground_truth.message_api)
        {
            message_api.equals_ground_truth(ground_truth_message_api)?;
        }

        if let (None, Some(_)) = (&self.reminder_api, &ground_truth.reminder_api) {
            Err("ReminderApi does not appear in the output but is expected by the ground truth")?;
        } else if let (Some(reminder_api), Some(ground_truth_reminder_api)) =
            (&self.reminder_api, &ground_truth.reminder_api)
        {
            reminder_api.equals_ground_truth(ground_truth_reminder_api)?;
        }

        if let (None, Some(_)) = (&self.food_platform, &ground_truth.food_platform) {
            Err("FoodPlatform does not appear in the output but is expected by the ground truth")?;
        } else if let (Some(food_platform), Some(ground_truth_food_platform)) =
            (&self.food_platform, &ground_truth.food_platform)
        {
            food_platform.equals_ground_truth(ground_truth_food_platform)?;
        }

        if let (None, Some(_)) = (&self.travel, &ground_truth.travel) {
            Err("Travel does not appear in the output but is expected by the ground truth")?;
        } else if let (Some(travel), Some(ground_truth_travel)) =
            (&self.travel, &ground_truth.travel)
        {
            travel.equals_ground_truth(ground_truth_travel)?;
        }
        Ok(())
    }
}
