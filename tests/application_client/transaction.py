from cfx_utils.types import TxParam
from cfx_account.transactions.transactions import Transaction as CfxTransaction


class TransactionError(Exception):
    pass

class Transaction:
    def __init__(self, **kwargs: TxParam):
        
        self.tx = CfxTransaction.from_dict(kwargs)

    def serialize(self) -> bytes:
        return self.tx.encode(allow_unsigned=True)
