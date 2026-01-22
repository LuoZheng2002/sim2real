#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PerturbationType {
    NoPerturbation,
    ActionA,         // same name, no description, no parameters
    ActionB,         // same name, ground truth description, no parameters
    ActionC,         // same name, no description, wrong parameters
    ActionD,         // same name ground truth description, wrong parameters
    ActionE,         // same name, other description, no parameters
    ActionRedundant, // add 2-3 similar but not identical distractor tools
    ObsParamDesc,    // perturb parameter description
    ObsParaphrase,   // paraphrase the query
    ObsToolDesc,     // perturb tool description
    ObsTypos,        // add typos to the query
    RewardCd,        // reward, cost in description
    RewardCdAb,      // reward, cost in description + tool name abbreviation
    RewardCdNt,      // reward, cost in description + neurtal tool name
    RewardTd,        // reward, time in description
    RewardTdAb,      // reward, time in description + tool name abbreviation
    RewardTdNt,      // reward, time in description + neutral tool name
    Transition,      // add "time-out, retry" to the first execution result
}

impl PerturbationType {
    pub fn all_perturbations() -> impl Iterator<Item = PerturbationType> {
        [
            PerturbationType::NoPerturbation,
            PerturbationType::ActionA,
            PerturbationType::ActionB,
            PerturbationType::ActionC,
            PerturbationType::ActionD,
            PerturbationType::ActionE,
            PerturbationType::ActionRedundant,
            PerturbationType::ObsParamDesc,
            PerturbationType::ObsParaphrase,
            PerturbationType::ObsToolDesc,
            PerturbationType::ObsTypos,
            PerturbationType::RewardCd,
            PerturbationType::RewardCdAb,
            PerturbationType::RewardCdNt,
            PerturbationType::RewardTd,
            PerturbationType::RewardTdAb,
            PerturbationType::RewardTdNt,
            PerturbationType::Transition,
        ]
        .into_iter()
    }
    pub fn to_folder_name(&self) -> String {
        let folder_name = match self {
            PerturbationType::NoPerturbation => "no_perturbation",
            PerturbationType::ActionA => "action_a",
            PerturbationType::ActionB => "action_b",
            PerturbationType::ActionC => "action_c",
            PerturbationType::ActionD => "action_d",
            PerturbationType::ActionE => "action_e",
            PerturbationType::ActionRedundant => "action_redundant",
            PerturbationType::ObsParamDesc => "obs_param_desc",
            PerturbationType::ObsParaphrase => "obs_paraphrase",
            PerturbationType::ObsToolDesc => "obs_tool_desc",
            PerturbationType::ObsTypos => "obs_typos",
            PerturbationType::RewardCd => "reward_cd",
            PerturbationType::RewardCdAb => "reward_cd_ab",
            PerturbationType::RewardCdNt => "reward_cd_nt",
            PerturbationType::RewardTd => "reward_td",
            PerturbationType::RewardTdAb => "reward_td_ab",
            PerturbationType::RewardTdNt => "reward_td_nt",
            PerturbationType::Transition => "transition",
        };
        folder_name.to_string()
    }
}
