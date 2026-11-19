import pytest
from application_client.boilerplate_command_sender import (
    CLA,
    P1,
    P2,
    BoilerplateCommandSender,
    Errors,
    InsType,
)
from application_client.boilerplate_response_unpacker import (
    unpack_get_public_key_response,
    unpack_sign_tx_response,
)
from application_client.boilerplate_transaction import Transaction
from application_client.utils import check_signature_validity
from ragger.error import ExceptionRAPDU
from ragger.navigator import NavIns, NavInsID

# In these tests we check the behavior of the device when asked to sign a transaction

# Touch position of the derivation path switch in the app settings of touch devices
SHOW_PATH_SWITCH_POSITION = (200, 113)


# In this test a transaction is sent to the device to be signed and validated on screen.
# The transaction is short and will be sent in one chunk.
# We will ensure that the displayed information is correct by using screenshots comparison.
def test_sign_tx_short_tx(backend, scenario_navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # The path used for this entire test
    path: str = "m/44'/1'/0'/0/0"

    # First we need to get the public key of the device in order to build the transaction
    rapdu = client.get_public_key(path=path)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    # Create the transaction that will be sent to the device for signing
    transaction = Transaction(
        nonce=1,
        coin="CRAB",
        value=777,
        to="de0b295669a9fd93d5f28d9ec85e40f4cb697bae",
        memo="For u EthDev",
    ).serialize()

    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(path=path, transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve()

    # The device as yielded the result, parse it and ensure that the signature is correct
    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)


def enable_show_derivation_path(device, navigator):
    if device.is_nano:
        instructions = [
            NavInsID.RIGHT_CLICK,  # App settings
            NavInsID.BOTH_CLICK,  # Enter settings, on the derivation path switch
            NavInsID.BOTH_CLICK,  # Toggle the switch
            NavInsID.RIGHT_CLICK,  # Back
            NavInsID.BOTH_CLICK,  # Leave settings, on App settings
            NavInsID.LEFT_CLICK,  # Home
        ]
    else:
        instructions = [
            NavInsID.USE_CASE_HOME_SETTINGS,
            NavIns(NavInsID.TOUCH, SHOW_PATH_SWITCH_POSITION),
            NavInsID.USE_CASE_SUB_SETTINGS_EXIT,
        ]
    navigator.navigate(
        instructions,
        screen_change_before_first_instruction=False,
        screen_change_after_last_instruction=False,
    )


# In this test the derivation path display is enabled in the app settings before signing.
# We will ensure that the path is displayed along all transaction fields by using screenshots comparison.
def test_sign_tx_show_path(backend, scenario_navigator, device, navigator):
    client = BoilerplateCommandSender(backend)
    path: str = "m/44'/1'/0'/0/0"

    rapdu = client.get_public_key(path=path)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    transaction = Transaction(
        nonce=1,
        coin="CRAB",
        value=777,
        to="de0b295669a9fd93d5f28d9ec85e40f4cb697bae",
        memo="For u EthDev",
    ).serialize()

    enable_show_derivation_path(device, navigator)

    with client.sign_tx(path=path, transaction=transaction):
        scenario_navigator.review_approve()

    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)


# In this test a transaction is sent to the device to be signed and validated on screen.
# This test is mostly the same as the previous one but with different values.
# In particular the long memo will force the transaction to be sent in multiple chunks
# def test_sign_tx_long_tx(firmware, backend, navigator, test_name):
def test_sign_tx_long_tx(backend, scenario_navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    path: str = "m/44'/1'/0'/0/0"

    rapdu = client.get_public_key(path=path)
    _, public_key, _, _ = unpack_get_public_key_response(rapdu.data)

    transaction = Transaction(
        nonce=1,
        coin="CRAB",
        value=666,
        to="de0b295669a9fd93d5f28d9ec85e40f4cb697bae",
        memo=(
            "This is a very long memo. "
            "It will force the app client to send the serialized transaction to be sent in chunk. "
            "As the maximum chunk size is 255 bytes we will make this memo greater than 255 characters. "
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed non risus. Suspendisse lectus tortor, "
            "dignissim sit amet, adipiscing nec, ultricies sed, dolor. Cras elementum ultrices diam."
        ),
    ).serialize()

    # Send the sign device instruction.
    # As it requires on-screen validation, the function is asynchronous.
    # It will yield the result when the navigation is done
    with client.sign_tx(path=path, transaction=transaction):
        # Validate the on-screen request by performing the navigation appropriate for this device
        scenario_navigator.review_approve()

    response = client.get_async_response().data
    _, der_sig, _ = unpack_sign_tx_response(response)
    assert check_signature_validity(public_key, der_sig, transaction)


# Transaction signature refused test
# The test will ask for a transaction signature that will be refused on screen
def test_sign_tx_refused(backend, scenario_navigator):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    path: str = "m/44'/1'/0'/0/0"

    rapdu = client.get_public_key(path=path)
    _, _, _, _ = unpack_get_public_key_response(rapdu.data)

    transaction = Transaction(
        nonce=1,
        coin="CRAB",
        value=666,
        to="de0b295669a9fd93d5f28d9ec85e40f4cb697bae",
        memo="This transaction will be refused by the user",
    ).serialize()

    with pytest.raises(ExceptionRAPDU) as e:
        with client.sign_tx(path=path, transaction=transaction):
            scenario_navigator.review_reject()

    # Assert that we have received a refusal
    assert e.value.status == Errors.SW_DENY
    assert len(e.value.data) == 0

    # The finished flow cannot be replayed without a new path chunk
    backend.wait_for_home_screen()
    with pytest.raises(ExceptionRAPDU) as e:
        backend.exchange(cla=CLA, ins=InsType.SIGN_TX, p1=P1.P1_START + 1, p2=P2.P2_LAST, data=transaction)
    assert e.value.status == Errors.SW_BAD_STATE
