use cosmwasm_std::Decimal;

pub const CONTRACT_NAME: &str = "eris-staking-hub";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONTRACT_DENOM: &str = "uluna";
pub const DAY: u64 = 24 * 60 * 60;

pub fn get_reward_fee_cap() -> Decimal {
    // 10% max reward fee
    Decimal::from_ratio(10_u128, 100_u128)
}
