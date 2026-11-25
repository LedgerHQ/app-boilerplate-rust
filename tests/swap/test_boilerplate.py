import struct

import pytest
from application_client.boilerplate_command_sender import (
    BoilerplateCommandSender,
)
from application_client.boilerplate_command_sender import (
    Errors as BoilerplateErrors,
)
from application_client.boilerplate_command_sender import (
    SwapAppErrorCode,
    SwapErrorCommonCode,
)
from application_client.boilerplate_currency_utils import BOL_PATH
from application_client.boilerplate_tokens import (
    TOKEN_DYNAMIC_OTHER_ADDRESS,
    TOKEN_DYNAMIC_USDT_ADDRESS,
    TOKEN_LINK_ADDRESS,
    TOKEN_USDC_ADDRESS,
    UNKNOWN_TOKEN_ADDRESS,
)
from application_client.boilerplate_transaction import TokenTransaction, Transaction
from ledger_app_clients.exchange.test_runner import (
    ALL_TESTS_EXCEPT_THORSWAP_AND_FEES,
    ExchangeTestRunner,
)
from ragger.error import ExceptionRAPDU

from . import cal_helper as cal

# Swap errors (common code, application code) that the app sends in the data of SW_SWAP_FAIL
WRONG_METHOD_ERROR = (SwapErrorCommonCode.ERROR_WRONG_METHOD, SwapAppErrorCode.DEFAULT)
WRONG_TOKEN_ERROR = (SwapErrorCommonCode.ERROR_WRONG_AMOUNT, SwapAppErrorCode.WRONG_TOKEN_INFO)
WRONG_FEES_ERROR = (SwapErrorCommonCode.ERROR_WRONG_FEES, SwapAppErrorCode.DEFAULT)
AMOUNT_CAST_ERROR = (SwapErrorCommonCode.ERROR_WRONG_AMOUNT, SwapAppErrorCode.AMOUNT_CAST_FAIL)
UNPARSABLE_TX_ERROR = (SwapErrorCommonCode.ERROR_GENERIC, SwapAppErrorCode.DEFAULT)


# Checks that a swap rejection carries the expected error code
def assert_swap_error(rapdu_error, expected_error):
    common_code, app_code = expected_error
    assert rapdu_error.status == BoilerplateErrors.SW_SWAP_FAIL
    assert rapdu_error.data[:2] == struct.pack("BB", common_code, app_code)


# ExchangeTestRunner implementation for Boilerplate.
# BoilerplateTests extends ExchangeTestRunner and in doing so inherits all necessary boilerplate and
# test management features.
# We only need to set the values we want to use for our test and the final payment request
class BoilerplateTests(ExchangeTestRunner):
    # The coin configuration of our currency. Replace by your own
    currency_configuration = cal.BOL_CURRENCY_CONFIGURATION
    # A valid template address of a supposed trade partner.
    valid_destination_1 = "de0b295669a9fd93d5f28d9ec85e40f4cb697bae"
    # A memo to use associated with the destination address if applicable.
    valid_destination_memo_1 = ""
    # A second valid template address of a supposed trade partner.
    valid_destination_2 = "9fc3da866e7dF3a1c57ade1a97c9f00a70f010c8"
    # A second memo to use associated with the destination address if applicable.
    valid_destination_memo_2 = "Boilerplate memo"
    # The address of the Speculos seed on the BOL_PATH.
    valid_refund = "fd2095a37e72be2cd575d18fe8f16e78c51eafa3"
    valid_refund_memo = ""

    # Values we ask the ExchangeTestRunner to use in the test setup
    valid_send_amount_1 = 1000
    valid_send_amount_2 = 666
    valid_fees_1 = 0
    valid_fees_2 = 0

    # Fake addresses to test the address rejection code.
    fake_refund = "abcdabcd"
    fake_refund_memo = "bla"
    fake_payout = "abcdabcd"
    fake_payout_memo = "bla"

    # The error code we expect our application to respond when encountering errors.
    signature_refusal_error_code = BoilerplateErrors.SW_DENY
    wrong_amount_error_code = BoilerplateErrors.SW_SWAP_FAIL
    wrong_destination_error_code = BoilerplateErrors.SW_SWAP_FAIL
    wrong_memo_error_code = BoilerplateErrors.SW_SWAP_FAIL

    # Asset of the final transaction, which must match the swapped currency
    final_tx_coin = cal.BOL_CURRENCY_CONFIGURATION.ticker
    # Token address of the final transaction, sent with SIGN_TOKEN_TX instead of SIGN_TX when set
    final_tx_token: str | None = None
    # Sends the token TX of final_tx_token with SIGN_TX, which cannot parse it
    final_tx_token_as_native = False
    # Error of a final TX transferring a token unknown to the app
    unknown_token_error = WRONG_METHOD_ERROR

    # The final transaction to craft and send as part of the SWAP finalization.
    # This function will be called by the ExchangeTestRunner in a callback like way
    def perform_final_tx(self, destination, send_amount, fees, memo):
        client = BoilerplateCommandSender(self.backend)
        if self.final_tx_token is None:
            tx = Transaction(nonce=1, coin=self.final_tx_coin, to=destination, value=send_amount, memo=memo).serialize()
            client.sign_tx_sync(path=BOL_PATH, transaction=tx)
        else:
            tx = TokenTransaction(nonce=1, token=self.final_tx_token, to=destination, value=send_amount, memo=memo).serialize()
            if self.final_tx_token_as_native:
                client.sign_tx_sync(path=BOL_PATH, transaction=tx)
            else:
                client.sign_token_tx_sync(path=BOL_PATH, transaction=tx)

        # TODO : assert signature validity. Not required but recommended

    # Performs a valid swap, then a final TX that the app must reject with a swap error. Set the
    # final_tx_* attributes beforehand to craft the final TX.
    def _perform_test_swap_rejected_final_tx(self, expected_error, fees=None):
        fees = self.valid_fees_1 if fees is None else fees
        self.perform_valid_swap_from_custom(
            self.valid_destination_1, self.valid_send_amount_1, fees, self.valid_destination_memo_1
        )
        with pytest.raises(ExceptionRAPDU) as e:
            self.perform_coin_specific_final_tx(
                self.valid_destination_1, self.valid_send_amount_1, fees, self.valid_destination_memo_1
            )
        assert_swap_error(e.value, expected_error)
        self.assert_exchange_is_started()

    # Boilerplate specific tests, run with the same mechanism as the generic ExchangeTestRunner tests

    # Test swap with a malicious TX transferring another asset than the swapped one
    def perform_test_swap_wrong_coin(self):
        self.final_tx_coin = "USDT"
        self._perform_test_swap_rejected_final_tx(WRONG_METHOD_ERROR)

    # Test swap of the native coin with a malicious TX transferring a known token instead
    def perform_test_swap_token_tx(self):
        self.final_tx_token = TOKEN_USDC_ADDRESS
        self._perform_test_swap_rejected_final_tx(WRONG_METHOD_ERROR)

    # Test swap with a malicious TX transferring a token unknown to the app
    def perform_test_swap_unknown_token_tx(self):
        self.final_tx_token = UNKNOWN_TOKEN_ADDRESS
        self._perform_test_swap_rejected_final_tx(self.unknown_token_error)

    # Test swap of a token with the native ticker and decimals, with a malicious TX transferring the native coin
    def perform_test_swap_token_with_native_ticker(self):
        self.currency_configuration = cal.BOL_CRAB_TOKEN_CURRENCY_CONFIGURATION
        self._perform_test_swap_rejected_final_tx(WRONG_METHOD_ERROR)

    # Test swap with fees, which Boilerplate transactions cannot carry
    def perform_test_swap_nonzero_fees(self):
        nonzero_fees = 1
        self._perform_test_swap_rejected_final_tx(WRONG_FEES_ERROR, fees=nonzero_fees)

    # Test swap of an amount beyond what a Boilerplate transaction can carry, with a TX matching its low bits
    def perform_test_swap_amount_overflow(self):
        u64_overflow = 1 << 64
        self.perform_valid_swap_from_custom(
            self.valid_destination_1,
            u64_overflow + self.valid_send_amount_1,
            self.valid_fees_1,
            self.valid_destination_memo_1,
        )
        with pytest.raises(ExceptionRAPDU) as e:
            self.perform_coin_specific_final_tx(
                self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
            )
        assert_swap_error(e.value, AMOUNT_CAST_ERROR)
        self.assert_exchange_is_started()


BOILERPLATE_SPECIFIC_TESTS = [
    "swap_wrong_coin",
    "swap_token_tx",
    "swap_unknown_token_tx",
    "swap_token_with_native_ticker",
    "swap_nonzero_fees",
    "swap_amount_overflow",
]


# Swap of a token of the hardcoded list (src/token/db.rs). The final TX is a SIGN_TOKEN_TX naming
# the token by its address: the app gives its ticker and decimals, which must be those of the coin
# configuration.
class BoilerplateTokenTests(BoilerplateTests):
    # USDC with 12 decimals, as in the hardcoded list
    currency_configuration = cal.BOL_USDC_CURRENCY_CONFIGURATION
    final_tx_token: str | None = TOKEN_USDC_ADDRESS
    unknown_token_error = WRONG_TOKEN_ERROR

    # Test token swap with a malicious TX transferring the native coin instead
    def perform_test_swap_native_tx(self):
        self.final_tx_token = None
        self._perform_test_swap_rejected_final_tx(WRONG_METHOD_ERROR)

    # Test token swap with a malicious native coin TX naming the swapped token as its coin
    def perform_test_swap_native_tx_with_token_ticker(self):
        self.final_tx_token = None
        self.final_tx_coin = self.currency_configuration.ticker
        self._perform_test_swap_rejected_final_tx(WRONG_METHOD_ERROR)

    # Test token swap with a malicious TX transferring another known token
    def perform_test_swap_token_mismatch(self):
        self.final_tx_token = TOKEN_LINK_ADDRESS
        self._perform_test_swap_rejected_final_tx(WRONG_TOKEN_ERROR)

    # Test token swap with a final TX that cannot be parsed: the token TX sent with SIGN_TX
    def perform_test_swap_unparsable_tx(self):
        self.final_tx_token_as_native = True
        self._perform_test_swap_rejected_final_tx(UNPARSABLE_TX_ERROR)


BOILERPLATE_TOKEN_SPECIFIC_TESTS = [
    "swap_native_tx",
    "swap_native_tx_with_token_ticker",
    "swap_token_mismatch",
    "swap_unknown_token_tx",
    "swap_nonzero_fees",
    "swap_amount_overflow",
    "swap_unparsable_tx",
]


# Swap of a token the app only knows from a token descriptor signed by the CAL (here by the test key
# of DYNAMIC_TOKEN_PARTNER). Exchange does not send the descriptor: the host sends the certificate of
# the CAL key and the descriptor to the app before the final TX. See doc/TOKENS.md.
class BoilerplateDynamicTokenTests(BoilerplateTokenTests):
    # USDT with 6 decimals, which only the descriptor gives
    currency_configuration = cal.BOL_DYNAMIC_USDT_CURRENCY_CONFIGURATION
    final_tx_token: str | None = TOKEN_DYNAMIC_USDT_ADDRESS
    # Descriptor sent before the final TX as (ticker, decimals, token address), None to send none
    descriptor: tuple[str, int, str] | None = ("USDT", 6, TOKEN_DYNAMIC_USDT_ADDRESS)

    def perform_final_tx(self, destination, send_amount, fees, memo):
        if self.descriptor is not None:
            ticker, decimals, token_address = self.descriptor
            client = BoilerplateCommandSender(self.backend)
            client.load_token_certificate()
            client.provide_dynamic_token(ticker=ticker, decimals=decimals, token_address=token_address)
        super().perform_final_tx(destination, send_amount, fees, memo)

    # Test token swap with the descriptor of a token with the swapped ticker and decimals, but at
    # another address than the token of the TX
    def perform_test_swap_descriptor_other_address(self):
        self.descriptor = ("USDT", 6, TOKEN_DYNAMIC_OTHER_ADDRESS)
        self._perform_test_swap_rejected_final_tx(WRONG_TOKEN_ERROR)

    # Test token swap of a token of the hardcoded list whose ticker and decimals a descriptor replaces
    def perform_test_swap_descriptor_overrides_listed_token(self):
        self.descriptor = ("USDT", 6, TOKEN_USDC_ADDRESS)
        self.final_tx_token = TOKEN_USDC_ADDRESS
        self.perform_test_swap_valid_1()

    # Test token swap without the descriptor of the token. The descriptors sent in the previous swaps
    # are not kept, as each swap starts the app anew.
    def perform_test_swap_no_descriptor(self):
        self.descriptor = None
        self._perform_test_swap_rejected_final_tx(WRONG_TOKEN_ERROR)

    # Test token swap with a descriptor giving the token other decimals than the coin configuration
    def perform_test_swap_descriptor_wrong_decimals(self):
        self.descriptor = ("USDT", 8, TOKEN_DYNAMIC_USDT_ADDRESS)
        self._perform_test_swap_rejected_final_tx(WRONG_TOKEN_ERROR)


# The token tests run for a descriptor token as well
BOILERPLATE_DYNAMIC_TOKEN_SPECIFIC_TESTS = BOILERPLATE_TOKEN_SPECIFIC_TESTS + [
    "swap_descriptor_other_address",
    "swap_descriptor_overrides_listed_token",
    "swap_no_descriptor",
    "swap_descriptor_wrong_decimals",
]


# We use a class to reuse the same Speculos instance (faster performances)
class TestsBoilerplate:
    # Run all the tests applicable to our setup: Thorswap / LiFi are not supported, and fees mismatch cannot be
    # tested as Boilerplate transactions carry no fees (covered by swap_nonzero_fees instead)
    @pytest.mark.parametrize("test_to_run", ALL_TESTS_EXCEPT_THORSWAP_AND_FEES + BOILERPLATE_SPECIFIC_TESTS)
    def test_boilerplate(self, backend, exchange_navigation_helper, test_to_run):
        # Call run_test method of ExchangeTestRunner
        BoilerplateTests(backend, exchange_navigation_helper).run_test(test_to_run)

    # Same tests for a swap of a token of the hardcoded list
    @pytest.mark.parametrize("test_to_run", ALL_TESTS_EXCEPT_THORSWAP_AND_FEES + BOILERPLATE_TOKEN_SPECIFIC_TESTS)
    def test_boilerplate_token(self, backend, exchange_navigation_helper, test_to_run):
        BoilerplateTokenTests(backend, exchange_navigation_helper).run_test(test_to_run)

    # Same tests for a swap of a token given by a signed token descriptor
    @pytest.mark.parametrize("test_to_run", ALL_TESTS_EXCEPT_THORSWAP_AND_FEES + BOILERPLATE_DYNAMIC_TOKEN_SPECIFIC_TESTS)
    def test_boilerplate_dynamic_token(self, backend, exchange_navigation_helper, test_to_run):
        BoilerplateDynamicTokenTests(backend, exchange_navigation_helper).run_test(test_to_run)
