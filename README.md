# ESDT-Staking-v2
This Smartcontract is able to stake ESDT to get same ESDT back as reward with adaptable apr

#### disclaimer: 
- using this code is on your own risk. This code is meant to be used for inspirational pruposes

To interact with the Smart Contract, you would need to have:
- python3.8 and upper
- newest installed erdpy (will be installed automatically after setting Workspace)
- [Pem wallet] ``` erdpy wallet derive wallet.pem ```
- [Elrond IDE](https://marketplace.visualstudio.com/items?itemName=Elrond.vscode-elrond-ide/) - extension for Visual Studio Code
```
ctrl+shift+P
Elrond: Setup Workspace
```
# Interaction
To interact with the Smart Contract, you would need to supply your wallet with some funds:
- [ESDT/EGLD faucet](https://r3d4.fr/elrond/devnet/)


        
# Contract build
example:
```
erdpy --verbose contract build "/home/project/staking"
```
# Deploying smartcontract through erdpy.json
There is stored whole configuration to get smartcontract deployed by using command. You will need to do this only once.
```
erdpy contract deploy
```
```
erdpy contract upgrade erd1qqqqqqqqqqqqqpgq6lsfc55vs5yk56mrnd5s95jgm9lrevvs0ztsrkernq
```
# Upgrading smartcontract
example:
```
erdpy --verbose contract upgrade erd1qqqqqqqqqqqqqpgq6lsfc55vs5yk56mrnd5s95jgm9lrevvs0ztsrkernq --recall-nonce --pem="wallet.pem" --gas-limit=60000000 --proxy="https://devnet-gateway.elrond.com" --chain=D --project=staking --arguments str:EFOO-8e80a5 25 259200 --send || return
```
# Arguments upon deployement
Upon deployement or while ugrading you have to set your arguments.
- ESDT token to be staked and supplied for rewards
```
"str:TokenIdentifier"
```
- APR
```
"25"
```
- Lock time for ESDT unstake in seconds
```
"259200"
```
```
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

```        
# Simulating contract call || mandos tests
- [Mandos tests](https://docs.elrond.com/developers/mandos-reference/structure/#docsNav)
- erdpy --verbose contract call
example
```
erdpy --verbose contract call erd1qqqqqqqqqqqqqpgq6lsfc55vs5yk56mrnd5s95jgm9lrevvs0ztsrkernq --function=ESDTTransfer --pem="wallet.pem" --proxy="https://devnet-gateway.elrond.com" --chain=D --recall-nonce --gas-limit=5000000 --arguments str:EFOO-8e80a5 1000000000000000000 str:stakeTokens --simulate
```
# Query SmartContract
Using view methods you are able to query your smartcontract to view informations.
- [query SmartContract-erdjs](https://github.com/ReneDuris/Query-SmartContract-erdjs)
       
 # View methods, storage mappers
- [View annotations](https://docs.elrond.com/developers/developer-reference/elrond-wasm-annotations/#endpoint-and-view)

Storage mappers can be used to store single values or multiple values. And with view method you are able to query stored value.
- [singleValueMapper](https://docs.elrond.com/developers/developer-reference/storage-mappers/#get)
```
    #[view(stakedAmount)]
    #[storage_mapper("stakedAmount")]
    fn staked_amount(&self, staker : &ManagedAddress) -> SingleValueMapper<BigUint>;
```
    
# Endpoints
- [Endpoint annotations](https://docs.elrond.com/developers/developer-reference/elrond-wasm-annotations/#endpoint-and-view)
- #[payable("*")] - throught the endpoint can be sent tokens, nfts
- #[only_owner] - only owner can call the endpoint
- #[endpoint] - annotation of endpoint which can be called
```
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
```
```
  #[endpoint(claimTokens)]
    fn claim(&self){
      let staker = &self.blockchain().get_caller();
      let token = self.save_token().get();
      let rewards = self.safe_rewards(&staker);
      self.send().direct_esdt(&staker, &token, 0u64, &rewards);

    }
```
# SmartContract API functions
[SmartContract API functions](https://docs.elrond.com/developers/developer-reference/elrond-wasm-api-functions/#docsNav)
```
let caller = self.blockchain().get_caller();
let current_timestamp = self.blockchain().get_block_timestamp();

```
# Calculation of rewards

```
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
    
    fn rps_calculated(&self) -> BigUint{
      let current_time = self.blockchain().get_block_timestamp();
      self.apr_last_time().set_if_empty(current_time);
      let apr_last_time = self.apr_last_time().get();
      let current_apr = self.apr().get();
      let diff_time = (current_time + 1u64) - apr_last_time;
      let rps_calculated = BigUint::from(current_apr *diff_time /100);
      rps_calculated
    }
    
    fn rps(&self){
      let current_time = self.blockchain().get_block_timestamp();
      let rps = self.rps_calculated();
      self.rps_acumulated().update(|amount| * amount += &rps);
      self.apr_last_time().set(current_time);
    }
```
