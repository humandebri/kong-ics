#定期的にallowanceを確認して、approveを叩く

from ic.identity import Identity
from ic.client import Client
from ic.agent import Agent
from ic.candid import encode, Types
# import numpy as np
# from functools import lru_cache
import time

def allowance(pid,pool):
    types = Types.Record({
            'account':Types.Record({'owner':Types.Principal}),
            'spender':Types.Record({'owner':Types.Principal}),})
    values = {
            'account':{'owner':pid},
            'spender':{'owner':pool},
    }
    params = [{'type': types, 'value': values}]
    return  encode(params)

def approve(amount, pool):

    types = Types.Record({
            'fee': Types.Opt(Types.Nat),
            'memo':Types.Opt(Types.Vec(Types.Nat8)),
            'from_subaccount':Types.Opt(Types.Vec(Types.Nat8)),
            'created_at_time':Types.Opt(Types.Nat64),
            'amount':Types.Nat,
            'spender':Types.Record({'owner':Types.Principal,'subaccount':Types.Opt(Types.Vec(Types.Nat8))}),
            })

    values = {
            'fee': [],
            'memo':[],
            'from_subaccount':[],
            'created_at_time':[time.time_ns()],
            'amount':amount,
            'spender':{'owner':pool,'subaccount':[]},
    }
    params = [{'type': types, 'value': values}]
    return  encode(params)


with open('infinity_identity.pem','r') as f:
                private_key_1 = f.read()
plug = Identity.from_pem(private_key_1)
client = Client(url = "https://ic0.app")
agent = Agent(plug, client)




token_list =[
{"name":"icp","icpswap" : "hkstf-6iaaa-aaaag-qkcoq-cai","sns": "ryjl3-tyaaa-aaaaa-aaaba-cai","fee":10_000,"sns_ikiti" : 100.1E6},
{"name":"usdc","icpswap" : "mohjv-bqaaa-aaaag-qjyia-cai","sns": "xevnm-gaaaa-aaaar-qafnq-cai","fee":10_000,"sns_ikiti" : 1_001E6},
{"name":"usdt","icpswap" : "hkstf-6iaaa-aaaag-qkcoq-cai","sns": 'cngnf-vqaaa-aaaar-qag4q-cai',"fee":10_000,"sns_ikiti" : 1_001E6},
{"name":"nicp","icpswap" : "e5a7x-pqaaa-aaaag-qkcga-cai","sns": 'buwm7-7yaaa-aaaar-qagva-cai',"fee":10_000,"sns_ikiti" : 100.1E8},
{"name":"kong","icpswap" : "ye4fx-gqaaa-aaaag-qnara-cai","sns": 'o7oak-iyaaa-aaaaq-aadzq-cai',"fee":10_000,"sns_ikiti" : 100_000E8},
{"name":"dkp","icpswap" : "ijd5l-jyaaa-aaaag-qdjga-cai","sns": 'zfcdd-tqaaa-aaaaq-aaaga-cai',"fee":100_000,"sns_ikiti" : 3_000_000E8},
{"name":"bob","icpswap" : "ybilh-nqaaa-aaaag-qkhzq-cai","sns": '7pail-xaaaa-aaaas-aabmq-cai',"fee":1_000_000,"sns_ikiti" : 1_001E8}
]

icp =  "ryjl3-tyaaa-aaaaa-aaaba-cai"
kong = "2ipq2-uqaaa-aaaar-qailq-cai"
my_pid = "xxol4-pnhff-brpzi-lxpfe-dy4hs-d2kzd-mif4k-aatnp-ltnhs-ydkcn-qqe"
icp_amount = 10000E8


while True:
    try:
        for _ in token_list:
            #check_allowance:kong icpもここで
            check_allowance = agent.query_raw(_["sns"], 'icrc2_allowance', allowance(my_pid, kong))
            print(check_allowance)

            if check_allowance[0]['value']['_3230440920']  < _["sns_ikiti"]*0.9:

                print(_["name"],check_allowance[0]['value']['_3230440920'],"kongのapproveが足りません！！",) 
                
                check_approve = agent.update_raw(_["sns"], 'icrc2_approve', approve(int(_["sns_ikiti"]), kong))
                print(check_approve) 
                
            #icpswap_check　icpとsns両方ここで認証する必要がある
            check_allowance = agent.query_raw(_["sns"], 'icrc2_allowance', allowance(my_pid,_["icpswap"]))
            print(check_allowance)

            if check_allowance[0]['value']['_3230440920'] < _["sns_ikiti"]*0.9:
                print(_["name"],check_allowance[0]['value']['_3230440920'],"icpsのapproveが足りません！！",) 
                check_approve = agent.update_raw(_["sns"], 'icrc2_approve', approve(int(_["sns_ikiti"]), _["icpswap"]))
                print(check_approve) 
                
                
            #icpのallowanceを設定 ほぼ動かない予定
            check_allowance = agent.query_raw(icp, 'icrc2_allowance', allowance(my_pid,_["icpswap"]))
            print(check_allowance)


            if check_allowance[0]['value']['_3230440920'] < icp_amount*0.5:
                
                print(_["name"],check_allowance[0]['value']['_3230440920'],"icpsのapproveが足りません！！",) 
                check_approve = agent.update_raw(icp, 'icrc2_approve', approve(int(icp_amount), _["icpswap"]))
                print(check_approve) 
            
            
        
        time.sleep(100)


    except:
        print('error')
        # message = f'kinic NAが止まりました！！！！'
        # tuti(message)
        time.sleep(300)


