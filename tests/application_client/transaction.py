import json
from dataclasses import dataclass
from .utils import UINT64_MAX

class TransactionError(Exception):
    pass

@dataclass
class Transaction:
    nonce: int
    value: int
    to: str
    gas: int
    gasPrice: int
    storageLimit: int
    epochHeight: int
    chainId: int
    data: str

    def serialize(self) -> bytes:
        if not 0 <= self.nonce <= UINT64_MAX:
            raise TransactionError(f"Bad nonce: '{self.nonce}'!")

        if len(self.to) != 40:
            raise TransactionError(f"Bad address: '{self.to}'!")

        # Serialize the transaction data to a JSON-formatted string
        return json.dumps({
            "nonce": self.nonce,
            "value": self.value,
            "to": self.to,
            "data": self.data,
            "gas": self.gas,
            "gasPrice": self.gasPrice,
            "storageLimit": self.storageLimit,
            "epochHeight": self.epochHeight,
            "chainId": self.chainId,
        }).encode('utf-8')
