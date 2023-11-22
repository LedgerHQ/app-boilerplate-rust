import json
from io import BytesIO
from typing import Union

from .boilerplate_utils import UINT64_MAX

class TransactionError(Exception):
    pass


class Transaction:
    def __init__(self,
                 nonce: int,
                 coin: str,
                 value: int,
                 to: str,
                 memo: str,
                 do_check: bool = True) -> None:
        self.nonce: int = nonce
        self.coin: str = coin
        self.value: str = value
        self.to: str = to 
        self.memo: str = memo

        if do_check:
            if not 0 <= self.nonce <= UINT64_MAX:
                raise TransactionError(f"Bad nonce: '{self.nonce}'!")

            if len(self.to) != 40:
                raise TransactionError(f"Bad address: '{self.to}'!")

    def serialize(self) -> bytes:
        # Serialize the transaction data to a JSON-formatted string
        return json.dumps({
            "nonce": self.nonce,
            "coin": self.coin,
            "value": self.value,
            "to": self.to,
            "memo": self.memo
        }).encode('utf-8')
