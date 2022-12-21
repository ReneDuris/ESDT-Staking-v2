#![no_std]
elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const YEAR_IN_SECONDS : u64 = 360 * 24 * 60 * 60;

#[elrond_wasm::contract]
pub trait Staking: 
      {
    #[init]
    fn init(&self,
      token: TokenIdentifier,
      apr: u64,
      locktime: u64
    ){
      require!(token.is_valid_esdt_identifier() == true, "Invalid ESDT");
      require!(apr > 1u64, "APR is set too low");
      require!(locktime > 1u64, "locktime is set too low");
      self.save_token().set(token);
      self.apr().set(apr);
      self.locktime().set(locktime);
      self.rps();
    }

    #[only_owner]
    #[endpoint(changeAPR)]
    fn change_apr(&self,apr: u64){
      require!(apr > 1u64, "APR is set too low");
      self.rps();
      self.apr().set(apr);
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(supplyRewards)]
    fn supply_rewards(&self){
      let saved_token = self.save_token().get();
      let payment = self.call_value().single_esdt();
      let token = payment.token_identifier;
      require!(token == saved_token, "Wrong token sent");
      let amount = payment.amount;
      self.supplied_rewards().update(|value| * value += amount);
    }

    #[payable("*")]
    #[endpoint(stakeTokens)]
    fn stake_tokens(&self){
      let payment = self.call_value().single_esdt();
      let saved_token = self.save_token().get();
      let value = payment.amount;
      let token = payment.token_identifier;
      require!(saved_token == token, "Wrong Token sent");
      require!(value >= 1u64, "Staked amount is too low");

      let staker = self.blockchain().get_caller();
      if self.staked_amount(&staker).get() > BigUint::zero() {
        let rewards = self.safe_rewards(&staker);
        self.storage_rewards(&staker).update(|amount| * amount += &rewards);
      }
      self.staked_amount(&staker).update(|amount| * amount += &value);
      self.total_staked().update(|amount| * amount += &value);
      self.save_position(&staker);
      self.lock_position(&staker);
      
      
    }

    #[payable("*")]
    #[endpoint(unstakeTokens)]
    fn unstake_tokens(&self,value: BigUint){
      let staker = self.blockchain().get_caller();
      let token = self.save_token().get();
      let staked_amount_mapper = self.staked_amount(&staker);
      require!(staked_amount_mapper.is_empty() == false, "Nothing staked");
      self.unlocked_position(&staker);
      let staked_amount = staked_amount_mapper.get();
      require!(value <= staked_amount, "You staked less token than you trying to unstake");

      let rewards = self.safe_rewards(&staker);

      staked_amount_mapper.update(|amount| * amount -= &value);

      let left_amount = &staked_amount - &value;
      if left_amount < 1u64{
        self.send().direct_esdt(&staker, &token, 0u64, &staked_amount);
        self.total_staked().update(|amount| * amount -= &staked_amount);
        
      }
      else{
        self.send().direct_esdt(&staker, &token, 0u64, &value);
        self.total_staked().update(|amount| * amount -= &value);
      }
      self.send().direct_esdt(&staker, &token, 0u64, &rewards);
      
    }

    #[endpoint(claimTokens)]
    fn claim(&self){
      let staker = &self.blockchain().get_caller();
      let token = self.save_token().get();
      let rewards = self.safe_rewards(&staker);
      self.send().direct_esdt(&staker, &token, 0u64, &rewards);

    }

    #[endpoint(reinvestTokens)]
    fn reinvest(&self){
      let staker = &self.blockchain().get_caller();
      let rewards = self.safe_rewards(&staker);
      self.staked_amount(&staker).update(|value| * value += &rewards);
      self.total_staked().update(|value| * value += &rewards);
    }
  
      fn safe_rewards(&self, staker: &ManagedAddress)-> BigUint{
      let supplied_rewards_mapper = self.supplied_rewards();
      let rewards = self.calculate_reward(&staker);
      require!(rewards <= supplied_rewards_mapper.get(), "Rewards are not available");
      supplied_rewards_mapper.update(|amount| * amount -= &rewards);
      self.lock_position(&staker);
      rewards
      }

    fn rps(&self){
      let current_time = self.blockchain().get_block_timestamp();
      let rps = self.rps_calculated();
      self.rps_acumulated().update(|amount| * amount += &rps);
      self.apr_last_time().set(current_time);
    }

    fn rps_calculated(&self) -> BigUint{
      let current_time = self.blockchain().get_block_timestamp();
      self.apr_last_time().set_if_empty(current_time);
      let apr_last_time = self.apr_last_time().get();
      let current_apr = self.apr().get();
      let diff_time = (current_time + 1u64) - apr_last_time;
      let rps_calculated = BigUint::from(current_apr *diff_time /100);
      rps_calculated
    }
  
    fn save_position(&self, staker : &ManagedAddress){
      let rps_acumulated = self.rps_acumulated().get();
      let rps = rps_acumulated + self.rps_calculated();
      self.new_position(staker).set(rps);

    }

    #[view(reward)]
    fn calculate_reward(&self, staker: &ManagedAddress) -> BigUint{
      let my_stake = self.is_not_empty(self.staked_amount(staker));
      let rps_position = self.is_not_empty(self.new_position(staker));
      let rps_acumulated = self.rps_acumulated().get();
      let rps = rps_acumulated + self.rps_calculated();
      let current_rps = rps - rps_position;
      let result = current_rps * my_stake / YEAR_IN_SECONDS;
      let storage_rewards_mapper = self.storage_rewards(staker);
      let mut storage_rewards = BigUint::zero();
      if storage_rewards_mapper.is_empty() == false{
        storage_rewards = self.storage_rewards(staker).take();
      }
      let rewards = result + storage_rewards;
      self.save_position(staker);
      rewards
    }    

    fn is_not_empty(&self,mapper:SingleValueMapper<BigUint>) -> BigUint{
      if mapper.is_empty() {
        sc_panic!("Nothing staked");
      }
      mapper.get()
    }

    fn lock_position(&self,staker: &ManagedAddress){
      let locktime = self.locktime().get();
      let current_time = self.blockchain().get_block_timestamp();
      let unlocktime = current_time + locktime;
      self.unlocktime(staker).set(unlocktime);
    }

    fn unlocked_position(&self, staker: &ManagedAddress){
      let current_time = self.blockchain().get_block_timestamp();
      let staker_unlocktime = self.unlocktime(staker).get();
      require!(staker_unlocktime < current_time, "Lock Time did not passed yet.");
    }
    #[view(totalStaked)]
    #[storage_mapper("totalStaked")]
    fn total_staked(&self) -> SingleValueMapper<BigUint>;

    #[view(suppliedRewards)]
    #[storage_mapper("suppliedRewards")]
    fn supplied_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(storageRewards)]
    #[storage_mapper("storageRewards")]
    fn storage_rewards(&self, staker : &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(stakedAmount)]
    #[storage_mapper("stakedAmount")]
    fn staked_amount(&self, staker : &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(stakedPosition)]
    #[storage_mapper("stakedPosition")]
    fn new_position(&self, staker : &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(RPSAcumulated)]
    #[storage_mapper("RPSAcumulated")]
    fn rps_acumulated(&self) -> SingleValueMapper<BigUint>;

    #[view(Token)]
    #[storage_mapper("Token")]
    fn save_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(APR)]
    #[storage_mapper("APR")]
    fn apr(&self) -> SingleValueMapper<u64>;

    #[view(lockTime)]
    #[storage_mapper("lockTime")]
    fn locktime(&self) -> SingleValueMapper<u64>;

    #[view(unlockTime)]
    #[storage_mapper("unlockTime")]
    fn unlocktime(&self, staker: &ManagedAddress) -> SingleValueMapper<u64>;

    #[view(APRLastTime)]
    #[storage_mapper("APRLastTime")]
    fn apr_last_time(&self) -> SingleValueMapper<u64>;
      }