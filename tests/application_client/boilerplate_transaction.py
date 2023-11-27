import json
from dataclasses import dataclass
from .boilerplate_utils import UINT64_MAX

class TransactionError(Exception):
    pass

@dataclass
class Transaction:
    nonce: int
    coin: str
    value: str
    to: str
    memo: str

    def serialize(self) -> bytes:
        if not 0 <= self.nonce <= UINT64_MAX:
            raise TransactionError(f"Bad nonce: '{self.nonce}'!")

        if len(self.to) != 40:
            raise TransactionError(f"Bad address: '{self.to}'!")

        # Serialize the transaction data to a JSON-formatted string
        return json.dumps({
            "nonce": self.nonce,
            "coin": self.coin,
            "value": self.value,
            "to": self.to,
            "memo": self.memo
        }).encode('utf-8')
