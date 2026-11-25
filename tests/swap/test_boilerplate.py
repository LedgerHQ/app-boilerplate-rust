import pytest
from application_client.boilerplate_command_sender import (
    BoilerplateCommandSender,
)
from application_client.boilerplate_command_sender import (
    Errors as BoilerplateErrors,
)
from application_client.boilerplate_currency_utils import BOL_PATH
from application_client.boilerplate_tokens import TOKEN_USDC_ADDRESS, UNKNOWN_TOKEN_ADDRESS
from application_client.boilerplate_transaction import TokenTransaction, Transaction
from ledger_app_clients.exchange.test_runner import (
    ALL_TESTS_EXCEPT_THORSWAP_AND_FEES,
    ExchangeTestRunner,
)
from ragger.error import ExceptionRAPDU

from . import cal_helper as cal


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

    # The final transaction to craft and send as part of the SWAP finalization.
    # This function will be called by the ExchangeTestRunner in a callback like way
    def perform_final_tx(self, destination, send_amount, fees, memo):
        client = BoilerplateCommandSender(self.backend)
        if self.final_tx_token is None:
            tx = Transaction(nonce=1, coin=self.final_tx_coin, to=destination, value=send_amount, memo=memo).serialize()
            client.sign_tx_sync(path=BOL_PATH, transaction=tx)
        else:
            tx = TokenTransaction(nonce=1, token=self.final_tx_token, to=destination, value=send_amount, memo=memo).serialize()
            client.sign_token_tx_sync(path=BOL_PATH, transaction=tx)

        # TODO : assert signature validity. Not required but recommended

    # Boilerplate specific tests, run with the same mechanism as the generic ExchangeTestRunner tests

    # Test swap with a malicious TX transferring another asset than the swapped one
    def perform_test_swap_wrong_coin(self):
        self.perform_valid_swap_from_custom(
            self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
        )
        self.final_tx_coin = "USDT"
        with pytest.raises(ExceptionRAPDU) as e:
            self.perform_coin_specific_final_tx(
                self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
            )
        assert e.value.status == BoilerplateErrors.SW_SWAP_FAIL
        self.assert_exchange_is_started()

    # Test swap of the native coin with a malicious TX transferring a known token instead
    def perform_test_swap_token_tx(self):
        self._perform_test_swap_token_instead_of_native(TOKEN_USDC_ADDRESS)

    # Test swap of the native coin with a malicious TX transferring an unknown token instead
    def perform_test_swap_unknown_token_tx(self):
        self._perform_test_swap_token_instead_of_native(UNKNOWN_TOKEN_ADDRESS)

    def _perform_test_swap_token_instead_of_native(self, token):
        self.perform_valid_swap_from_custom(
            self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
        )
        self.final_tx_token = token
        with pytest.raises(ExceptionRAPDU) as e:
            self.perform_coin_specific_final_tx(
                self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
            )
        assert e.value.status == BoilerplateErrors.SW_SWAP_FAIL
        self.assert_exchange_is_started()

    # Test swap of a token with the native ticker and decimals, with a malicious TX transferring the native coin
    def perform_test_swap_token_with_native_ticker(self):
        self.currency_configuration = cal.BOL_CRAB_TOKEN_CURRENCY_CONFIGURATION
        self.perform_valid_swap_from_custom(
            self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
        )
        with pytest.raises(ExceptionRAPDU) as e:
            self.perform_coin_specific_final_tx(
                self.valid_destination_1, self.valid_send_amount_1, self.valid_fees_1, self.valid_destination_memo_1
            )
        assert e.value.status == BoilerplateErrors.SW_SWAP_FAIL
        self.assert_exchange_is_started()

    # Test swap with fees, which Boilerplate transactions cannot carry
    def perform_test_swap_nonzero_fees(self):
        nonzero_fees = 1
        self.perform_valid_swap_from_custom(
            self.valid_destination_1, self.valid_send_amount_1, nonzero_fees, self.valid_destination_memo_1
        )
        with pytest.raises(ExceptionRAPDU) as e:
            self.perform_coin_specific_final_tx(
                self.valid_destination_1, self.valid_send_amount_1, nonzero_fees, self.valid_destination_memo_1
            )
        assert e.value.status == BoilerplateErrors.SW_SWAP_FAIL
        self.assert_exchange_is_started()

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
        assert e.value.status == BoilerplateErrors.SW_SWAP_FAIL
        self.assert_exchange_is_started()


BOILERPLATE_SPECIFIC_TESTS = [
    "swap_wrong_coin",
    "swap_token_tx",
    "swap_unknown_token_tx",
    "swap_token_with_native_ticker",
    "swap_nonzero_fees",
    "swap_amount_overflow",
]


# We use a class to reuse the same Speculos instance (faster performances)
class TestsBoilerplate:
    # Run all the tests applicable to our setup: Thorswap / LiFi are not supported, and fees mismatch cannot be
    # tested as Boilerplate transactions carry no fees (covered by swap_nonzero_fees instead)
    @pytest.mark.parametrize("test_to_run", ALL_TESTS_EXCEPT_THORSWAP_AND_FEES + BOILERPLATE_SPECIFIC_TESTS)
    def test_boilerplate(self, backend, exchange_navigation_helper, test_to_run):
        # Call run_test method of ExchangeTestRunner
        BoilerplateTests(backend, exchange_navigation_helper).run_test(test_to_run)
