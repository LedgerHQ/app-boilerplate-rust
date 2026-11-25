from application_client.boilerplate_currency_utils import BOL_PATH
from ledger_app_clients.exchange.cal_helper import CurrencyConfiguration
from ragger.bip import pack_derivation_path
from ragger.utils import create_currency_config

# Define a configuration for each currency used in our tests: native coins and tokens

# BOL token currency definition
BOL_CONF = create_currency_config("CRAB", "Rust Boilerplate", sub_coin_config=None)
# Serialized derivation path for the Boilerplate app
BOL_PACKED_DERIVATION_PATH = pack_derivation_path(BOL_PATH)
# Coin configuration mock as stored in CAL for the SWAP feature
BOL_CURRENCY_CONFIGURATION = CurrencyConfiguration(
    ticker="CRAB", conf=BOL_CONF, packed_derivation_path=BOL_PACKED_DERIVATION_PATH
)

# A CAL token with the native ticker and decimals. Its sub coin configuration is not empty, so it is
# a token: only an empty sub coin configuration designates the native coin
BOL_CRAB_TOKEN_CONF = create_currency_config("CRAB", "Rust Boilerplate", sub_coin_config=("CRAB", 9))
BOL_CRAB_TOKEN_CURRENCY_CONFIGURATION = CurrencyConfiguration(
    ticker="CRAB", conf=BOL_CRAB_TOKEN_CONF, packed_derivation_path=BOL_PACKED_DERIVATION_PATH
)

# BOL USDC token configuration - USDC with 12 decimals as defined in src/token/db.rs
BOL_USDC_CONF = create_currency_config("USDC", "Boilerplate", sub_coin_config=("USDC", 12))
BOL_USDC_PACKED_DERIVATION_PATH = BOL_PACKED_DERIVATION_PATH
BOL_USDC_CURRENCY_CONFIGURATION = CurrencyConfiguration(
    ticker="USDC",
    conf=BOL_USDC_CONF,
    packed_derivation_path=BOL_USDC_PACKED_DERIVATION_PATH,
)

# BOL Dynamic USDT token configuration - USDT with 6 decimals provided via CAL
BOL_DYNAMIC_USDT_CONF = create_currency_config("USDT", "Boilerplate", sub_coin_config=("USDT", 6))
BOL_DYNAMIC_USDT_PACKED_DERIVATION_PATH = BOL_PACKED_DERIVATION_PATH
BOL_DYNAMIC_USDT_CURRENCY_CONFIGURATION = CurrencyConfiguration(
    ticker="USDT",
    conf=BOL_DYNAMIC_USDT_CONF,
    packed_derivation_path=BOL_DYNAMIC_USDT_PACKED_DERIVATION_PATH,
)
