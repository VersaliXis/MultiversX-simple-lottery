# MX Lottery Contract
Contract allowing to create a lottery, buy tickets and draw a winner.

## Table of Contents
- [Concept](#concept)
- [For Developers](#developers)

## Concept
This is the simplest lottery you can have. Participents: 
- Contract Owner ("Owner")
- Lottery Creator ("Creator")
- Lottery Participant A ("A")
- Lottery Participant B ("B")

### 1. New Lottery
The **Creator** wants to create a lottery in order for people to win an amazing price. He wants also earn EGLD because once prize is distributed, he earns the EGLD earned by the contract by selling tickets.
- He calls the `createLottery` endpoint with parameters 
    - `ticket_price: BigUint` the ticket price in EGLD
    - `deadline: u64` the UNIX timestamp of the deadline, after which the lottery will be closed
- His transaction must also contains an ESDT transfert. It can be an NFT or an ESDT token. This payment will be used by the contract as the price.

### 2. Buying tickets
The **Participant** can decide to buy one one more tickets in a single transaction, *if the lottery is still opened*. The more they have tickets, the better are their chance to win the prize.
- They call the `buy_tickets` endpoint with parameters:
    - `lottery_id: u64` the unique id of the lottery
    - `opt_quantity: u32` *Optionnal* the number of tickets to buy. If not provided, buy 1 ticket only.
- Their transaction must also be an EGLD payment, with value equals to `ticket_price * opt_quantity`

### 3. Distribute Prize
- When the lottery is closed, anyone can call the `callRewards` endpoint with parameters:
    - `lottery_id: u64` the unique id of the lottery
- The contract perform call another contract in charge of picking a random int that will be the wining ticket index.
- Then the contract distribute the prize to the **Winner**, and the EGLD quantity accumulated by selling tickets to the **Creator**.
- If no ticket has been sold, then the contract send back to the **Creator** his prize
> Why calling another contract? Because *randomness can be predicted on MX* based on the current state, and anyone can simulate the transaction until it provides the desired result.  A way to avoid predictability is to perform an asynchronous call to another dedicated contract **on another shard**.


## Developers

### 1. Installation
Clone the repository:
``` bash
git clone https://github.com/VersaliXis/MultiversX-simple-lottery.git
```
### 2. Deployment
In order to deploy a Lottery follow this steps:
1. Deploy the `random-picker` contract on a first shard (e.g.  1)
2. Deploy the `lottery` contract on **another shard** (e.g. 2), passing the `random-picker`'s contract address.

### 3. Upgrading
1. Upgrade or Deploy another vertion of the `random-picker` contract on a first shard (e.g.  1)
2. Upgrade the `lottery` contract on **another shard** (e.g. 2), passing the `random-picker`'s new or previously existing address.


If you want to improve the project, you can install [MXOps](https://github.com/Catenscia/MxOps) in order to perform automated contract calls. 
