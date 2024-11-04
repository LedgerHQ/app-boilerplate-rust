from tests.application_client.transaction import Transaction

tx = Transaction(nonce=1, value=777, to="de0b295669a9fd93d5f28d9ec85e40f4cb697bae", maxPriorityFeePerGas=1000000000, maxFeePerGas=1000000000, gas=1000000, storageLimit=1000000, epochHeight=1000000, chainId=1, data="For u EthDev".encode("utf-8"))
print(tx.serialize())
