import hashlib
import struct

import pytest
from application_client.boilerplate_command_sender import CLA, P1, P2, BoilerplateCommandSender, Errors, InsType
from application_client.boilerplate_currency_utils import BOL_PATH
from application_client.boilerplate_response_unpacker import (
    unpack_get_public_key_response,
    unpack_sign_tx_response,
)
from application_client.boilerplate_token_descriptor import (
    DESCRIPTOR_VERSION,
    HARDENED,
    SLIP44_COIN_TYPE,
    TAG_MAGNITUDE,
    build_token_descriptor,
    build_tuid,
)
from application_client.boilerplate_tokens import (
    TOKEN_DYNAMIC_OTHER_ADDRESS,
    TOKEN_DYNAMIC_USDT_ADDRESS,
    TOKEN_USDC_ADDRESS,
)
from application_client.boilerplate_transaction import TokenTransaction, Transaction
from application_client.tlv import format_tlv
from application_client.utils import check_signature_validity
from ecdsa import SECP256k1, SigningKey
from ecdsa.util import sigencode_der
from ragger.bip import pack_derivation_path
from ragger.error import ExceptionRAPDU

# In these tests we check the behavior of the device when it receives token descriptors signed by
# the Ledger CAL, here mocked by a test key whose certificate only exists on Speculos.
# Each test starts a new app: no certificate is loaded and no descriptor is known.

DESTINATION = "de0b295669a9fd93d5f28d9ec85e40f4cb697bae"

USDT_TICKER = "USDT"
USDT_DECIMALS = 6

# Bounds of the ticker and decimals the app accepts
MAX_TICKER_LEN = 12
MAX_DECIMALS = 30

ETHEREUM_COIN_TYPE = 60


def token_transaction(token_address: str, value: int = 1_500_000, memo: str = "Token transfer") -> bytes:
    return TokenTransaction(nonce=1, token=token_address, value=value, to=DESTINATION, memo=memo).serialize()


# Sends a token transfer that the app must reject without review, as it does not know the token
def assert_unknown_token(client: BoilerplateCommandSender, token_address: str) -> None:
    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_token_tx_sync(path=BOL_PATH, transaction=token_transaction(token_address))
    assert e.value.status == Errors.SW_TX_PARSING_FAIL


# Signs a transaction, approving its review, and checks the signature.
# It then waits for the home screen: the app rejects APDUs while the "Transaction signed" status is displayed.
def sign_and_check(client, scenario_navigator, sign, transaction: bytes, test_name=None, do_comparison=True) -> None:
    rapdu = client.get_public_key(path=BOL_PATH)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    with sign(path=BOL_PATH, transaction=transaction):
        scenario_navigator.review_approve(test_name=test_name, do_comparison=do_comparison)

    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)
    client.backend.wait_for_home_screen()


def sign_token_tx(client, scenario_navigator, transaction: bytes, test_name=None, do_comparison=True) -> None:
    sign_and_check(client, scenario_navigator, client.sign_token_tx, transaction, test_name, do_comparison)


# A descriptor makes a token unknown to the app signable, with the ticker and decimals it gives
def test_provide_token_info_new_token(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)
    assert_unknown_token(client, TOKEN_DYNAMIC_USDT_ADDRESS)

    client.load_token_certificate()
    rapdu = client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)
    assert len(rapdu.data) == 0

    # 500.25 USDT
    transaction = token_transaction(TOKEN_DYNAMIC_USDT_ADDRESS, value=500_250_000, memo="USDT transfer")
    sign_token_tx(client, scenario_navigator, transaction)


# The descriptor comes before the hardcoded list: it replaces the ticker and decimals of a listed token
def test_provide_token_info_override_hardcoded(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_USDC_ADDRESS)

    # Displayed as 1.5 USDT, where the hardcoded USDC with 12 decimals would display 0.0000015 USDC
    transaction = token_transaction(TOKEN_USDC_ADDRESS, value=1_500_000, memo="USDC address as USDT")
    sign_token_tx(client, scenario_navigator, transaction)


# A listed token keeps its own ticker in a descriptor: only other addresses cannot reuse it
def test_provide_token_info_hardcoded_ticker_same_address(backend):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    usdc_decimals = 12
    rapdu = client.provide_dynamic_token("USDC", usdc_decimals, TOKEN_USDC_ADDRESS)
    assert len(rapdu.data) == 0


# The descriptor accepts the coin type of the app hardened as well
def test_provide_token_info_hardened_coin_type(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS, coin_type=SLIP44_COIN_TYPE | HARDENED)
    sign_token_tx(client, scenario_navigator, token_transaction(TOKEN_DYNAMIC_USDT_ADDRESS), do_comparison=False)


# The descriptor is kept across signing flows, native ones included
def test_provide_token_info_lasts_across_transactions(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)

    sign_token_tx(client, scenario_navigator, token_transaction(TOKEN_DYNAMIC_USDT_ADDRESS), do_comparison=False)

    native_transaction = Transaction(nonce=2, coin="CRAB", value=777, to=DESTINATION, memo="Native transfer").serialize()
    sign_and_check(client, scenario_navigator, client.sign_tx, native_transaction, do_comparison=False)

    sign_token_tx(client, scenario_navigator, token_transaction(TOKEN_DYNAMIC_USDT_ADDRESS), do_comparison=False)


# The app keeps one descriptor: a new one replaces the previous one
def test_provide_token_info_replaces_previous(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)
    other_decimals = 8
    client.provide_dynamic_token("OTHER", other_decimals, TOKEN_DYNAMIC_OTHER_ADDRESS)

    sign_token_tx(client, scenario_navigator, token_transaction(TOKEN_DYNAMIC_OTHER_ADDRESS), do_comparison=False)
    assert_unknown_token(client, TOKEN_DYNAMIC_USDT_ADDRESS)


# Ticker, decimals and value of each bound case
BOUND_CASES = {
    "max_ticker": ("A" * MAX_TICKER_LEN, USDT_DECIMALS, 1_500_000),
    "zero_decimals": ("NFT", 0, 42),
    # The smallest amount has the most digits: 0. followed by 30 decimals
    "max_decimals": ("TINY", MAX_DECIMALS, 1),
}


# The longest ticker and the extreme decimals are displayed in full
@pytest.mark.parametrize("case", BOUND_CASES)
def test_provide_token_info_bounds(backend, scenario_navigator, test_name, case):
    ticker, decimals, value = BOUND_CASES[case]
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(ticker, decimals, TOKEN_DYNAMIC_USDT_ADDRESS)

    transaction = token_transaction(TOKEN_DYNAMIC_USDT_ADDRESS, value=value, memo=f"{ticker} transfer")
    sign_token_tx(client, scenario_navigator, transaction, test_name=f"{test_name}_{case}")


def sign_with_unknown_key(data: bytes) -> bytes:
    key = SigningKey.generate(curve=SECP256k1, hashfunc=hashlib.sha256)
    return key.sign_deterministic(data, hashfunc=hashlib.sha256, sigencode=sigencode_der)


def tampered_descriptor() -> bytes:
    # The decimals are changed after signing
    signed_decimals = USDT_DECIMALS
    tampered_decimals = 18
    descriptor = build_token_descriptor(USDT_TICKER, signed_decimals, TOKEN_DYNAMIC_USDT_ADDRESS)
    signed_magnitude = format_tlv(TAG_MAGNITUDE, signed_decimals)
    assert descriptor.count(signed_magnitude) == 1
    return descriptor.replace(signed_magnitude, format_tlv(TAG_MAGNITUDE, tampered_decimals))


UNKNOWN_TUID_TAG = 0x11
TOKEN_ADDRESS = bytes.fromhex(TOKEN_DYNAMIC_USDT_ADDRESS)
EXTRA_ADDRESS_BYTE = struct.pack(">B", 0)


# Each descriptor below is rejected. The slot is emptied before the descriptor is checked, so the
# valid descriptor sent first for the same token is discarded as well.
@pytest.mark.parametrize(
    "overrides",
    [
        {"coin_type": ETHEREUM_COIN_TYPE},
        # The name of the C boilerplate app
        {"app_name": "Boilerplate"},
        {"version": DESCRIPTOR_VERSION + 1},
        {"ticker": ""},
        {"ticker": "A" * (MAX_TICKER_LEN + 1)},
        {"ticker": "US DT"},
        {"ticker": "USDÉ"},
        # The review identifies an asset by its ticker: other assets' tickers are rejected, ignoring case
        {"ticker": "CRAB"},
        {"ticker": "crab"},
        {"ticker": "USDC"},
        {"ticker": "usdc"},
        {"decimals": MAX_DECIMALS + 1},
        {"tuid": b""},
        {"tuid": build_tuid(TOKEN_ADDRESS) + format_tlv(UNKNOWN_TUID_TAG, 0)},
        {"tuid": format_tlv(UNKNOWN_TUID_TAG, TOKEN_ADDRESS)},
        {"tuid": build_tuid(TOKEN_ADDRESS[:-1])},
        {"tuid": build_tuid(TOKEN_ADDRESS + EXTRA_ADDRESS_BYTE)},
        {"sign": sign_with_unknown_key},
    ],
    ids=[
        "wrong_coin_type",
        "wrong_app_name",
        "wrong_version",
        "empty_ticker",
        "ticker_too_long",
        "ticker_with_space",
        "ticker_non_ascii",
        "native_ticker",
        "native_ticker_lowercase",
        "hardcoded_ticker",
        "hardcoded_ticker_lowercase",
        "too_many_decimals",
        "tuid_empty",
        "tuid_unknown_tag",
        "tuid_without_address",
        "tuid_address_too_short",
        "tuid_address_too_long",
        "signed_with_unknown_key",
    ],
)
def test_provide_token_info_rejected(backend, overrides):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)

    fields = {"ticker": USDT_TICKER, "decimals": USDT_DECIMALS, "token_address": TOKEN_DYNAMIC_USDT_ADDRESS}
    fields.update(overrides)
    with pytest.raises(ExceptionRAPDU) as e:
        client.provide_token_info(build_token_descriptor(**fields))
    assert e.value.status == Errors.SW_INVALID_DYNAMIC_TOKEN
    assert len(e.value.data) == 0

    assert_unknown_token(client, TOKEN_DYNAMIC_USDT_ADDRESS)


# A descriptor whose content no longer matches its signature, or that is not a descriptor, is rejected
@pytest.mark.parametrize(
    "descriptor",
    [tampered_descriptor(), b""],
    ids=["tampered", "empty"],
)
def test_provide_token_info_invalid_payload(backend, descriptor):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)

    with pytest.raises(ExceptionRAPDU) as e:
        client.provide_token_info(descriptor)
    assert e.value.status == Errors.SW_INVALID_DYNAMIC_TOKEN

    assert_unknown_token(client, TOKEN_DYNAMIC_USDT_ADDRESS)


# Without the certificate of its signing key, a descriptor cannot be verified
def test_provide_token_info_without_certificate(backend):
    client = BoilerplateCommandSender(backend)
    with pytest.raises(ExceptionRAPDU) as e:
        client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)
    assert e.value.status == Errors.SW_INVALID_DYNAMIC_TOKEN

    assert_unknown_token(client, TOKEN_DYNAMIC_USDT_ADDRESS)


# A descriptor ends the pending signing flow: the token of a flow cannot change once it started
def test_provide_token_info_ends_pending_flow(backend):
    client = BoilerplateCommandSender(backend)
    client.load_token_certificate()
    backend.exchange(cla=CLA, ins=InsType.SIGN_TOKEN_TX, p1=P1.P1_START, p2=P2.P2_MORE, data=pack_derivation_path(BOL_PATH))
    client.provide_dynamic_token(USDT_TICKER, USDT_DECIMALS, TOKEN_DYNAMIC_USDT_ADDRESS)

    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(
            cla=CLA,
            ins=InsType.SIGN_TOKEN_TX,
            p1=P1.P1_START + 1,
            p2=P2.P2_LAST,
            data=token_transaction(TOKEN_DYNAMIC_USDT_ADDRESS),
        )
    assert e.value.status == Errors.SW_BAD_STATE
