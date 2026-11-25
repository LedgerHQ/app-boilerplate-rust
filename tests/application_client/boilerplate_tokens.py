# Token addresses used by the tests, hex encoded

# Hardcoded token list of the app (src/token/db.rs), by ticker
TOKEN_USDC_ADDRESS = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"  # 12 decimals
TOKEN_WETH_ADDRESS = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"  # 14 decimals
TOKEN_DAI_ADDRESS = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"  # 14 decimals
TOKEN_BTC_ADDRESS = "112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00"  # 12 decimals
TOKEN_LINK_ADDRESS = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100"  # 14 decimals

HARDCODED_TOKEN_ADDRESSES = {
    "USDC": TOKEN_USDC_ADDRESS,
    "WETH": TOKEN_WETH_ADDRESS,
    "DAI": TOKEN_DAI_ADDRESS,
    "BTC": TOKEN_BTC_ADDRESS,
    "LINK": TOKEN_LINK_ADDRESS,
}

# Token the app does not know
UNKNOWN_TOKEN_ADDRESS = "00ffeeddccbbaa99887766554433221100ffeeddccbbaa998877665544332211"

# Token absent from the hardcoded list, only known through a signed token descriptor
TOKEN_DYNAMIC_USDT_ADDRESS = "cafebabedeadbeefcafebabedeadbeefcafebabedeadbeefcafebabedeadbeef"
# Another token only known through a signed token descriptor
TOKEN_DYNAMIC_OTHER_ADDRESS = "1111111111111111111111111111111111111111111111111111111111111111"
