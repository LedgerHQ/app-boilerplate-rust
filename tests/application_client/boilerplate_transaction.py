import json
from dataclasses import dataclass

from .boilerplate_utils import UINT64_MAX


class TransactionError(Exception):
    pass


@dataclass
class Transaction:
    nonce: int
    coin: str
    value: int
    to: str
    memo: str

    def serialize(self) -> bytes:
        if not 0 <= self.nonce <= UINT64_MAX:
            raise TransactionError(f"Bad nonce: '{self.nonce}'!")

        if len(self.to) != 40:
            raise TransactionError(f"Bad address: '{self.to}'!")

        # Serialize the transaction data to a JSON-formatted string
        return json.dumps(
            {
                "nonce": self.nonce,
                "coin": self.coin,
                "value": self.value,
                "to": self.to,
                "memo": self.memo,
            }
        ).encode("utf-8")


# Token transfer, sent with SIGN_TOKEN_TX. The token is named by its 32-byte address only: the app
# resolves its ticker and decimals.
@dataclass
class TokenTransaction:
    nonce: int
    token: str
    value: int
    to: str
    memo: str

    def serialize(self) -> bytes:
        if not 0 <= self.nonce <= UINT64_MAX:
            raise TransactionError(f"Bad nonce: '{self.nonce}'!")

        if len(self.token) != 64:
            raise TransactionError(f"Bad token address: '{self.token}'!")

        if len(self.to) != 40:
            raise TransactionError(f"Bad address: '{self.to}'!")

        return json.dumps(
            {
                "nonce": self.nonce,
                "token": self.token,
                "value": self.value,
                "to": self.to,
                "memo": self.memo,
            }
        ).encode("utf-8")
