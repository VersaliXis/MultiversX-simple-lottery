#![no_std]
//can we return data after endpoint call?
mod random_picker_proxy;

use multiversx_sc::imports::*;
multiversx_sc::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi, Debug, PartialEq)]
pub enum LotteryStatus {
    Opened,
    Closed,
    Failed,
    Completed
}
// Ticket Object
// Use timestamp to ensure each ticket is unique (setmapper doesn't allow duplicates)
#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct Ticket<M: ManagedTypeApi> {
    pub owner: ManagedAddress<M>,
    pub lottery_id: u32,
    pub timestamp: u64,
    pub id: u32,
}

// Lottery Object
#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct LotteryObj<M: ManagedTypeApi> {
    pub creator: ManagedAddress<M>,
    pub prize_token: TokenIdentifier<M>,
    pub token_nonce: u64,
    pub prize: BigUint<M>,
    pub ticket_price: BigUint<M>,
    pub deadline: u64,
    pub status: LotteryStatus,
}
#[multiversx_sc::contract]
pub trait Lottery {
    // #################   proxy    #################

    #[proxy]
    fn random_picker_proxy(&self, sc_address: ManagedAddress) -> random_picker_proxy::Proxy<Self::Api>;

    // #################   storage    #################


    /// Address of the random picker contract
    #[view(getRandomPickerAddress)]
    #[storage_mapper("random_picker_address")]
    fn random_picker_address(&self) -> SingleValueMapper<ManagedAddress>;

    /// Lottery per lottery index
    #[view(getLottery)]
    #[storage_mapper("lottery")]
    fn lottery(&self, lottery_id: &u32) -> SingleValueMapper<LotteryObj<Self::Api>>;

    // All opened or unclaimed lotteries ID
    #[view(getLotteriesId)]
    #[storage_mapper("lotteriesId")]
    fn lotteries_id(&self) -> SetMapper<u32>;

    /// Last lottery ID
    #[view(getLastLotteryId)]
    #[storage_mapper("lastLotteryId")]
    fn last_lottery_id(&self) -> SingleValueMapper<u32>;

    /// Lottery Status
    #[view(getLotteryStatus)]
    fn get_lottery_status(&self, lottery_id: u32) -> LotteryStatus{
        require!(self.lotteries_id().contains(&lottery_id), "lottery id unavailable");
        return self.lottery(&lottery_id).get().status
    }

    /// Tickets per lottery
    #[view(getTickets)]
    #[storage_mapper("tickets")]
    fn tickets(&self, lottery_id: &u32) -> SetMapper<Ticket<Self::Api>>;

    // #################   init && upgrade    #################

    #[init]
    fn init(&self, random_picker_address: ManagedAddress) {
        self.last_lottery_id().set(0);
        self.random_picker_address().set(random_picker_address);
    }

    /// Allows to change picker's contract address on upgrade
    #[upgrade]
    fn upgrade(&self, random_picker_address: ManagedAddress) {
        self.random_picker_address().set(random_picker_address);
    }


    // #################   endpoints    #################

    /// Allow a user to create a Lottery
    /// 
    /// ### Arguments
    /// 
    /// * **ticket_price** - `BigUint` Price per ticket in EGLD
    /// * **deadline** - `u64` Date after which lottery is closed in UNIX
    /// 
    /// ### Payments
    /// 
    /// * **prize_payment** : Single ESDT payment becomes the prize for this lottery
    /// 
    /// ### Returns
    /// 
    /// * **lottery_id** - `u32` The unique id of the newly generated lottery
    #[endpoint(createLottery)]
    #[payable("*")]
    fn create_lottery(&self, ticket_price: BigUint, deadline: u64) -> u32{
        let timestamp = self.blockchain().get_block_timestamp();
        require!(timestamp < deadline, "deadline has already passed");
        require!(ticket_price > 0, "ticket price must be > 0");

        let creator = self.blockchain().get_caller();
        let prize_payment = self.call_value().single_esdt(); //error if no payment
        let prize_token = prize_payment.token_identifier;
        let prize = prize_payment.amount;
        let token_nonce = prize_payment.token_nonce;
        let new_lottery = LotteryObj {
            creator, 
            prize_token, token_nonce, prize, 
            ticket_price, 
            deadline, 
            status: LotteryStatus::Opened
        };
        //updates directly last id then returns it
        let lottery_id = self.last_lottery_id().update(|id| {
            *id += 1;
            *id
        });

        //save data
        self.lottery(&lottery_id).set(new_lottery);
        self.lotteries_id().insert(lottery_id);
        return lottery_id
    }

    /// Allow a User to enter buy one or more lottery ticket for a given lottery
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** - `u32` The unique id of the lottery
    /// 
    /// * **opt_quantity** - `OptionalValue<u32>` The quantity of ticket to buy. 1 by default.
    /// 
    /// 
    #[endpoint(buyTickets)]
    #[payable("EGLD")]
    fn buy_tickets(&self, lottery_id: u32, opt_quantity: OptionalValue<u32>) {
        require!(self.lotteries_id().contains(&lottery_id), "unavailable lottery");
        let quantity = match opt_quantity {
            OptionalValue::Some(qty) => qty,
            OptionalValue::None => 1,
        };
        require!(quantity >= 1, "ticket quantity must be >= 1");
        
        let amount = self.call_value().egld_value().clone_value();
        self.update_lottery_status(&lottery_id);
        let lottery = self.lottery(&lottery_id).get();

        require!(lottery.status == LotteryStatus::Opened, "lottery is closed");
        let required_payment = lottery.ticket_price * quantity;

        require!(amount >= required_payment, "ticket price doesn't match");
        let timestamp = self.blockchain().get_block_timestamp();

        //= means inclusive end
        // Use both timestamp and id to make tickets unique
        for i in 1..= quantity {
            let owner = self.blockchain().get_caller();
            let new_ticket = Ticket {owner, lottery_id: lottery_id.clone(), timestamp, id:i};
            self.tickets(&lottery_id).insert(new_ticket);
        }        
    }

    /// Allow anyone to distribute prize. Outcome can be:
    /// - Lottery is succesful, ticket receip sent to Creator and prize sent to Winner
    /// - Lottery failed: no ticket has been sold, prize sent back to Creator
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** -`u32` The unique lottery id
    /// 
    #[endpoint(callRewards)]
    fn call_rewards(&self, lottery_id: u32) {
        require!(lottery_id > 0 && self.lotteries_id().contains(&lottery_id), "lottery id unavailable");

        let lottery_status = self.update_lottery_status(&lottery_id);

        // IMPORTANT: status Completed is used so that prize cannot be ditributed two times
        require!(lottery_status == LotteryStatus::Closed, "lottery is not closed or is already completed");

        self.call_random_picker_async(&lottery_id)
    }

    // #################   restricted endpoints    #################

    // #################   functions    #################

    /// Update lottery status. For now, just check if deadline has passed or not
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** -`u32` The unique lottery id
    /// 
    /// ### Returns
    /// * **new_lottery_status** -`LotteryStatus` The new lottery status
    /// 
    fn update_lottery_status(&self, lottery_id: &u32) -> LotteryStatus{
        let lottery = self.lottery(&lottery_id).get();
        let current_time = self.blockchain().get_block_timestamp();
        let old_status = lottery.status;
        match old_status {
            LotteryStatus::Opened => {
                if current_time > lottery.deadline {
                    self.set_lottery_status(&lottery_id, LotteryStatus::Closed);
                    LotteryStatus::Closed
                } else {
                    old_status
                }
            },
            _ => old_status,
        }
    }

    /// Set a new lottery status. 
    /// Called by update_lottery_status, or can be called by another function
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** -`u32` The unique lottery id
    /// 
    /// * **new_status** -`LotteryStatus` The new status to update to
    /// 
    fn set_lottery_status(&self, lottery_id: &u32, new_status: LotteryStatus){
        let mut lottery = self.lottery(&lottery_id).get();
        lottery.status = new_status;
    }

    /// Call to the picker contract
    /// Must be asynch (so contracts on different shards) in order to ensure randomness unpredictability
    /// Called by the endpoint callReward
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** -`u32` The unique lottery id
    /// 
    /// * **new_status** -`LotteryStatus` The new status to update to
    /// 
    fn call_random_picker_async(&self, lottery_id: &u32) {
        let proxy_address = self.random_picker_address().get();
        let mut proxy_instance = self.random_picker_proxy(proxy_address);
        let mut lottery = self.lottery(&lottery_id).get();
        let tickets_mapper = self.tickets(&lottery_id); 
        let max_index = tickets_mapper.len();

        if max_index > 0{
            proxy_instance
                .random_pick_index(max_index)
                .with_callback(self.callbacks().pick_winner(&lottery_id))
                .async_call_and_exit();
        } else {
            let token_identifier = lottery.prize_token;
            let token_nonce = lottery.token_nonce;
            let amount = lottery.prize;
            // Send back prize
            self.tx()
                .to(lottery.creator)
                .payment(EsdtTokenPayment::new(token_identifier, token_nonce, amount))
                .transfer();

            lottery.status = LotteryStatus::Failed;
        }
        
    }

    /// Call to the picker contract
    /// Must be asynch (so contracts on different shards) in order to ensure randomness unpredictability
    /// Called by the callback pick_winner
    /// 
    /// IMPORTANT: status Completed is used so that prize cannot be ditributed two times
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** -`u32` The unique lottery id
    /// 
    /// * **winning_ticket** -`Ticket<Self::Api>` The wining ticket that has been picked
    /// 
    fn distribute_prize(&self, lottery_id: &u32, winning_ticket: Ticket<Self::Api>) {
        let mut lottery = self.lottery(&lottery_id).get();
        
        // Send prize to the winner
        let token_nonce = lottery.token_nonce;
        let token_identifier = lottery.prize_token;
        let amount = lottery.prize;
        self.tx()
            .to(winning_ticket.owner)
            .payment(EsdtTokenPayment::new(token_identifier, token_nonce, amount))
            .transfer();
        
        // Send the income due to ticket selling to the lottery creator
        let lottery_creator = lottery.creator;
        let ticket_price = lottery.ticket_price;
        let ticket_qty = BigUint::from(self.tickets(&lottery_id).len());
        let amount = &ticket_qty * &ticket_price;
        self.send().direct_egld(&lottery_creator, &amount);

        lottery.status = LotteryStatus::Completed;
    }
    
    // #################   callbacks    #################

    /// Callback for the random-picker contract 
    /// Called by the random-picker contract
    /// 
    /// ### Arguments
    /// 
    /// * **lottery_id** -`u32` The unique lottery id
    /// 
    #[callback]
    fn pick_winner(&self, 
        lottery_id: &u32, 
        #[call_result] result: ManagedAsyncCallResult<usize>
    ) {
        match result {
            ManagedAsyncCallResult::Ok(picked_index) => {
                let tickets_mapper = self.tickets(&lottery_id); 
                let tickets_iter = tickets_mapper.iter().enumerate();
                for (i, ticket) in tickets_iter {
                    if picked_index == i {
                       self.distribute_prize(&lottery_id ,ticket) //returns the winning ticket
                    }
                }
                
            },
            ManagedAsyncCallResult::Err(_err) => {panic!("error occure")},
        };
    }    
}
