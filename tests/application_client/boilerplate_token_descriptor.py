from collections.abc import Callable

from .signing_partners import DYNAMIC_TOKEN_PARTNER
from .tlv import format_tlv

# Token descriptor, sent with PROVIDE_TOKEN_INFO. It is a TLV signed by the Ledger CAL, here
# mocked by DYNAMIC_TOKEN_PARTNER, whose certificate must be loaded on the device beforehand.

# Descriptor tags
TAG_STRUCTURE_TYPE = 0x01
TAG_VERSION = 0x02
TAG_COIN_TYPE = 0x03
TAG_APP_NAME = 0x04
TAG_TICKER = 0x05
TAG_MAGNITUDE = 0x06
TAG_TUID = 0x07
TAG_SIGNATURE = 0x08

# TUID tag of the token address, the only TUID tag the app accepts
TUID_TAG_TOKEN_ADDRESS = 0x10

STRUCTURE_TYPE_DYNAMIC_TOKEN = 0x90
DESCRIPTOR_VERSION = 1
# SLIP-0044 coin type of the app, accepted hardened or not
SLIP44_COIN_TYPE = 1
HARDENED = 0x80000000
# Name of the app in Cargo.toml, which the descriptor must carry
DESCRIPTOR_APP_NAME = "Rust Boilerplate"


def build_tuid(token_address: bytes) -> bytes:
    return format_tlv(TUID_TAG_TOKEN_ADDRESS, token_address)


def build_token_descriptor(
    ticker: str,
    decimals: int,
    token_address: str,
    coin_type: int = SLIP44_COIN_TYPE,
    app_name: str = DESCRIPTOR_APP_NAME,
    version: int = DESCRIPTOR_VERSION,
    tuid: bytes | None = None,
    sign: Callable[[bytes], bytes] = DYNAMIC_TOKEN_PARTNER.sign,
) -> bytes:
    """Builds a signed descriptor of the token at `token_address`, hex encoded.

    `tuid` replaces the TUID built from `token_address`, and `sign` the signing key.
    """
    if tuid is None:
        tuid = build_tuid(bytes.fromhex(token_address))

    descriptor = format_tlv(TAG_STRUCTURE_TYPE, STRUCTURE_TYPE_DYNAMIC_TOKEN)
    descriptor += format_tlv(TAG_VERSION, version)
    descriptor += format_tlv(TAG_COIN_TYPE, coin_type)
    descriptor += format_tlv(TAG_APP_NAME, app_name)
    descriptor += format_tlv(TAG_TICKER, ticker)
    descriptor += format_tlv(TAG_MAGNITUDE, decimals)
    descriptor += format_tlv(TAG_TUID, tuid)
    # The signature covers every TLV before it
    descriptor += format_tlv(TAG_SIGNATURE, sign(descriptor))
    return descriptor
