import pytest
from application_client.boilerplate_command_sender import BoilerplateCommandSender, Errors
from application_client.boilerplate_currency_utils import BOL_PATH
from application_client.boilerplate_response_unpacker import (
    unpack_get_public_key_response,
    unpack_sign_tx_response,
)
from application_client.boilerplate_tokens import (
    HARDCODED_TOKEN_ADDRESSES,
    TOKEN_BTC_ADDRESS,
    TOKEN_DAI_ADDRESS,
    UNKNOWN_TOKEN_ADDRESS,
)
from application_client.boilerplate_transaction import TokenTransaction
from application_client.utils import check_signature_validity
from ragger.error import ExceptionRAPDU

from .test_sign_cmd import enable_show_derivation_path

# In these tests we check the behavior of the device when asked to sign a token transaction.
# The token is named by its address only: the device displays the ticker and decimals it knows for it.

DESTINATION = "de0b295669a9fd93d5f28d9ec85e40f4cb697bae"


# Sign a transfer of the first, a middle and the last token of the hardcoded list.
# We will ensure that the token ticker and the amount with the token decimals are displayed.
@pytest.mark.parametrize("ticker", ["USDC", "WETH", "LINK"])
def test_sign_token_tx(backend, scenario_navigator, test_name, ticker):
    client = BoilerplateCommandSender(backend)

    rapdu = client.get_public_key(path=BOL_PATH)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    # The same base units are displayed differently by tokens with different decimals:
    # 150 USDC (12 decimals), 1.5 WETH and 1.5 LINK (14 decimals)
    value = 15 * 10**13
    transaction = TokenTransaction(
        nonce=1,
        token=HARDCODED_TOKEN_ADDRESSES[ticker],
        value=value,
        to=DESTINATION,
        memo=f"{ticker} transfer",
    ).serialize()

    with client.sign_token_tx(path=BOL_PATH, transaction=transaction):
        scenario_navigator.review_approve(test_name=f"{test_name}_{ticker}")

    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)


# A token the app does not know cannot be displayed, so its transfer is rejected without review
def test_sign_token_tx_unknown_token(backend):
    client = BoilerplateCommandSender(backend)
    transaction = TokenTransaction(
        nonce=1,
        token=UNKNOWN_TOKEN_ADDRESS,
        value=1000,
        to=DESTINATION,
        memo="Unknown token",
    ).serialize()

    with pytest.raises(ExceptionRAPDU) as e:
        client.sign_token_tx_sync(path=BOL_PATH, transaction=transaction)
    assert e.value.status == Errors.SW_TX_PARSING_FAIL
    assert len(e.value.data) == 0


# The user refuses the token transfer on screen
def test_sign_token_tx_refused(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)
    transaction = TokenTransaction(
        nonce=1,
        token=TOKEN_DAI_ADDRESS,
        value=10**16,
        to=DESTINATION,
        memo="This transaction will be refused by the user",
    ).serialize()

    with pytest.raises(ExceptionRAPDU) as e:
        with client.sign_token_tx(path=BOL_PATH, transaction=transaction):
            scenario_navigator.review_reject()
    assert e.value.status == Errors.SW_DENY
    assert len(e.value.data) == 0


# The long memo forces the transaction to be sent in several chunks
def test_sign_token_tx_long_memo(backend, scenario_navigator):
    client = BoilerplateCommandSender(backend)

    rapdu = client.get_public_key(path=BOL_PATH)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    transaction = TokenTransaction(
        nonce=10,
        token=TOKEN_BTC_ADDRESS,
        value=5 * 10**11,
        to=DESTINATION,
        memo=(
            "This is a very long memo for a token transaction. "
            "It will force the app client to send the serialized transaction in chunks. "
            "As the maximum chunk size is 255 bytes we will make this memo greater than 255 characters. "
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed non risus. Suspendisse lectus tortor."
        ),
    ).serialize()

    with client.sign_token_tx(path=BOL_PATH, transaction=transaction):
        scenario_navigator.review_approve()

    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)


# With the derivation path display enabled, the review shows every optional field: the token and the path
def test_sign_token_tx_show_path(backend, scenario_navigator, device, navigator):
    client = BoilerplateCommandSender(backend)

    rapdu = client.get_public_key(path=BOL_PATH)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    transaction = TokenTransaction(
        nonce=1,
        token=TOKEN_DAI_ADDRESS,
        value=10**16,
        to=DESTINATION,
        memo="DAI transfer",
    ).serialize()

    enable_show_derivation_path(device, navigator)

    with client.sign_token_tx(path=BOL_PATH, transaction=transaction):
        scenario_navigator.review_approve()

    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)
