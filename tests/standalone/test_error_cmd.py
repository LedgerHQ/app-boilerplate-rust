import json
import struct

import pytest
from application_client.boilerplate_command_sender import CLA, P1, P2, BoilerplateCommandSender, Errors, InsType
from application_client.boilerplate_currency_utils import BOL_PATH
from application_client.boilerplate_transaction import Transaction
from ragger.bip import pack_derivation_path
from ragger.error import ExceptionRAPDU

DESTINATION = "de0b295669a9fd93d5f28d9ec85e40f4cb697bae"


# Ensure the app returns an error when a bad CLA is used
def test_bad_cla(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA + 1, ins=InsType.GET_VERSION)
    assert e.value.status == Errors.SW_CLA_NOT_SUPPORTED


# Ensure the app returns an error when a bad INS is used
def test_bad_ins(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=0xFF)
    assert e.value.status == Errors.SW_INS_NOT_SUPPORTED


# Ensure the app returns an error when a bad P1 or P2 is used
def test_wrong_p1p2(backend):
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_VERSION, p1=P1.P1_START + 1, p2=P2.P2_LAST)
    assert e.value.status == Errors.SW_WRONG_P1P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_VERSION, p1=P1.P1_START, p2=P2.P2_MORE)
    assert e.value.status == Errors.SW_WRONG_P1P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_APP_NAME, p1=P1.P1_START + 1, p2=P2.P2_LAST)
    assert e.value.status == Errors.SW_WRONG_P1P2
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.GET_APP_NAME, p1=P1.P1_START, p2=P2.P2_MORE)
    assert e.value.status == Errors.SW_WRONG_P1P2


# Ensure the app returns an error when a bad data length is used
def test_wrong_data_length(backend):
    # APDUs must be at least 4 bytes: CLA, INS, P1, P2.
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange_raw(bytes.fromhex("E00300"))
    assert e.value.status == Errors.SW_WRONG_APDU_LENGTH
    # APDUs advertises a too long length
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange_raw(bytes.fromhex("E003000005"))
    assert e.value.status == Errors.SW_WRONG_APDU_LENGTH


# The SIGN_TX flow must start with the path chunk before any transaction chunk
def test_sign_tx_continuation_without_start(backend):
    transaction = Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize()
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE


# SIGN_TX chunks must be strictly consecutive
def test_sign_tx_skipped_chunk(backend):
    transaction = Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize()
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(BOL_PATH))
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 2, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE
    # The rejected sequence must also have discarded the flow
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE


# A flow that failed on its final chunk cannot be resumed
def test_sign_tx_no_resume_after_error(backend):
    invalid_transaction = b"not a transaction"
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(BOL_PATH))
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=invalid_transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=invalid_transaction)
    assert e.value.status == Errors.SW_BAD_STATE


# A path chunk that fails to parse does not start a flow
def test_sign_tx_no_flow_after_invalid_path(backend):
    path_length = 5
    truncated_path = struct.pack(">BI", path_length, 0x8000002C)
    transaction = Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize()
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=truncated_path)
    assert e.value.status == Errors.SW_WRONG_APDU_LENGTH
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE


# Any rejected command ends a pending flow, whether rejected while decoding its header or by its handler
@pytest.mark.parametrize(
    "ins, p1, p2, data, status",
    [
        (InsType.SIGN_TX, P1.P1_START, P2.P2_LAST, pack_derivation_path(BOL_PATH), Errors.SW_WRONG_P1P2),
        (InsType.GET_PUBLIC_KEY, P1.P1_START, P2.P2_LAST, pack_derivation_path("m/44'/60'/0'/0/0"), Errors.SW_INVALID_PATH),
    ],
    ids=["sign_tx_wrong_p1p2", "get_public_key_error"],
)
def test_sign_tx_no_resume_after_rejected_command(backend, ins, p1, p2, data, status):
    transaction = Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize()
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(BOL_PATH))
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=ins, p1=p1, p2=p2, data=data)
    assert e.value.status == status
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE


# Fields that are not part of the reviewed transaction are rejected, as they would be signed unseen
def test_sign_tx_unknown_field(backend):
    client = BoilerplateCommandSender(backend)
    transaction = json.loads(Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize())
    transaction["fee"] = 1000
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=json.dumps(transaction).encode())
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# Data trailing the transaction object is rejected
def test_sign_tx_trailing_data(backend):
    client = BoilerplateCommandSender(backend)
    transaction = Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize()
    trailing_data = b'{"to": "00"}'
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction + trailing_data)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# Text fields are restricted to printable ASCII, as anything else cannot be displayed faithfully
@pytest.mark.parametrize("field", ["coin", "memo"])
@pytest.mark.parametrize(
    "raw_text",
    [b"\x00", b"\n", "\u00e9".encode()],
    ids=["nul", "line_feed", "non_ascii"],
)
def test_sign_tx_undisplayable_text(backend, field, raw_text):
    client = BoilerplateCommandSender(backend)
    placeholder = "PLACEHOLDER"
    fields = {"nonce": 1, "coin": "CRAB", "value": 777, "to": DESTINATION, "memo": "memo"}
    fields[field] = placeholder
    # JSON serialization would escape these characters, so they are inserted raw afterwards
    transaction = Transaction(**fields).serialize().replace(placeholder.encode(), raw_text)
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# The coin is the unit of the displayed amount and cannot be empty
def test_sign_tx_empty_coin(backend):
    client = BoilerplateCommandSender(backend)
    transaction = Transaction(nonce=1, coin="", value=777, to=DESTINATION, memo="memo").serialize()
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# A path outside the scope of the app does not start a flow
def test_sign_tx_path_not_allowed(backend):
    other_coin_path = "m/44'/60'/0'/0/0"
    transaction = Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize()
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(other_coin_path))
    assert e.value.status == Errors.SW_INVALID_PATH
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE


# JSON escape sequences are rejected, as they would be displayed undecoded while consumers decode them
@pytest.mark.parametrize("field", ["coin", "memo"])
def test_sign_tx_escaped_text(backend, field):
    client = BoilerplateCommandSender(backend)
    placeholder = "PLACEHOLDER"
    escaped_text = b"Pay \\u0041lice"
    fields = {"nonce": 1, "coin": "CRAB", "value": 777, "to": DESTINATION, "memo": "memo"}
    fields[field] = placeholder
    transaction = Transaction(**fields).serialize().replace(placeholder.encode(), escaped_text)
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# Outside swap mode, only the native coin can be signed: other assets have no known decimals
def test_sign_tx_unsupported_coin(backend):
    client = BoilerplateCommandSender(backend)
    transaction = Transaction(nonce=1, coin="USDT", value=777, to=DESTINATION, memo="memo").serialize()
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# The accumulated transaction is limited to MAX_TRANSACTION_LEN bytes
def test_sign_tx_max_length(backend):
    max_transaction_len = 510
    chunk_len = 255
    filler = b"A"
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(BOL_PATH))
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_MORE, data=filler * chunk_len)
    # Reaching the limit is accepted: the content is only rejected once parsed
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TX,
            p1=P1.P1_START + 2,
            p2=P2.P2_LAST,
            data=filler * (max_transaction_len - chunk_len),
        )
    assert e.value.status == Errors.SW_TX_PARSING_FAIL

    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(BOL_PATH))
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_MORE, data=filler * chunk_len)
    backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 2, p2=P2.P2_MORE, data=filler * chunk_len)
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 3, p2=P2.P2_LAST, data=filler)
    assert e.value.status == Errors.SW_WRONG_TX_LENGTH


# The value is an unsigned 64-bit integer
@pytest.mark.parametrize("value", [1 << 64, -1, 1.5], ids=["overflow", "negative", "float"])
def test_sign_tx_invalid_value(backend, value):
    client = BoilerplateCommandSender(backend)
    transaction = Transaction(nonce=1, coin="CRAB", value=value, to=DESTINATION, memo="memo").serialize()
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# A field given twice is rejected, as the two values could be read differently
def test_sign_tx_duplicate_field(backend):
    client = BoilerplateCommandSender(backend)
    fields = json.loads(Transaction(nonce=1, coin="CRAB", value=777, to=DESTINATION, memo="memo").serialize())
    serialized_fields = [f"{json.dumps(name)}: {json.dumps(value)}" for name, value in fields.items()]
    duplicated_value = f'"value": {fields["value"] * 1000}'
    transaction = ("{" + ", ".join([*serialized_fields, duplicated_value]) + "}").encode()
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL
