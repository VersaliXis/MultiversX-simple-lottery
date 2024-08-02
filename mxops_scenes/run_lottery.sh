# Must run it at the root
# Set deadline in two minutes
export DEADLINE=$(($(date +%s)+60*1))


export TICKET_PRICE=1000000000000000
export PRIZE_TOKEN_ID="USDC-350c4e"
export PRIZE_TOKEN_AMOUNT=1000000 

export QTY_BUYER1=10
export QTY_BUYER2=20

export PRICE_BUYER1=$(($QTY_BUYER1*$TICKET_PRICE))
export PRICE_BUYER2=$(($QTY_BUYER2*$TICKET_PRICE))
echo $PRICE_BUYER1
echo $DEADLINE
mxops execute \
        -n devnet \
        -s lottery \
        ./mxops_scenes/accounts/devnet.yaml \
        ./mxops_scenes/03_lottery_run.yaml
