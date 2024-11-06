## GET_APP_INFO

#### Request format

| CLA    | INS    | P1     | P2     | Lc     |
| ------ | ------ | ------ | ------ | ------ |
| `0xe0` | `0x01` | `0x00` | `0x00` | `0x00` |

##### Request payload

None.

#### Response format

| Description   | Length |
| ------------- | ------ |
| Flags         | 1      |
| Major version | 1      |
| Minor version | 1      |
| Patch version | 1      |

Flags: `b000000YX`

- `X`: blind signing enabled
- `Y`: detailed display enabled

#### Examples

**Command**: `e001000000`

| CLA    | INS    | P1     | P2     | Lc     |
| ------ | ------ | ------ | ------ | ------ |
| `0xe0` | `0x01` | `0x00` | `0x00` | `0x00` |

**Response**: `03000002 9000`, blind signing enabled, detailed display enabled, application version: `0.0.2`.

## GET_PUBLIC_KEY

#### Request format

| CLA  | INS  | P1                    | P2                      | Lc       | Le       |
| ---- | ---- | --------------------- | ----------------------- | -------- | -------- |
| `E0` | `02` | `00`: no display      | `00`: no chain code     | variable | variable |
|      |      | `01`: display address | `01`: return chain code |          |          |

##### Request payload

| Description                                      | Length |
| ------------------------------------------------ | ------ |
| Number of BIP 32 derivations to perform (max 10) | 1      |
| First derivation index (big endian)              | 4      |
| ...                                              | 4      |
| Last derivation index (big endian)               | 4      |
| Chain ID (required when `P1 == 01`)              | 4      |

#### Response format

| Description             | Length |
| ----------------------- | ------ |
| Public key length       | 1      |
| Uncompressed public key | var    |
| Chain code length       | 1      |
| Chain code              | var    |

#### Examples

**Command**: `e002000015058000002c800001f7800000000000000000000000`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                            |
| ------ | ------ | ------ | ------ | ------ | ------------------------------------------------------------- |
| `0xe0` | `0x02` | `0x00` | `0x00` | `0x15` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000` |

`44'/503'/0'/0/0` is encoded as `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000`.

**Response**: `41 047b88d05ba40b8e6ed961b526ab68c7051d2a8602862c788f84416cc37e9c0a5c4213b20660a6591cd53ad81d5b68499acb835ac7a08c88e18bf8f4998061eb4a 9000`

---

**Command**: `e002000115058000002c800001f7800000000000000000000000`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                            |
| ------ | ------ | ------ | ------ | ------ | ------------------------------------------------------------- |
| `0xe0` | `0x02` | `0x00` | `0x01` | `0x15` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000` |

**Response**: `41 047b88d05ba40b8e6ed961b526ab68c7051d2a8602862c788f84416cc37e9c0a5c4213b20660a6591cd53ad81d5b68499acb835ac7a08c88e18bf8f4998061eb4a 20 20b19d018f0bf5264aa6a0953a22d2cc432205fc022adfeb0160b1cad0b4ab8b 9000`

---

**Command**: `e002010019058000002c800001f780000000000000000000000000000405`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                                       |
| ------ | ------ | ------ | ------ | ------ | ------------------------------------------------------------------------ |
| `0xe0` | `0x02` | `0x01` | `0x00` | `0x19` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000 0x00000405` |

Notice the chain ID (`0x00000405 ~ 1029`, aka mainnet) at the end.

**Response**: `41 047b88d05ba40b8e6ed961b526ab68c7051d2a8602862c788f84416cc37e9c0a5c4213b20660a6591cd53ad81d5b68499acb835ac7a08c88e18bf8f4998061eb4a 9000`

---

**Command**: `e002010119058000002c800001f780000000000000000000000000000405`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                                       |
| ------ | ------ | ------ | ------ | ------ | ------------------------------------------------------------------------ |
| `0xe0` | `0x02` | `0x01` | `0x01` | `0x19` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000 0x00000405` |

**Response**: `41 047b88d05ba40b8e6ed961b526ab68c7051d2a8602862c788f84416cc37e9c0a5c4213b20660a6591cd53ad81d5b68499acb835ac7a08c88e18bf8f4998061eb4a 20 20b19d018f0bf5264aa6a0953a22d2cc432205fc022adfeb0160b1cad0b4ab8b 9000`

## SIGN_TX

#### Request format

| CLA  | INS  | P1                                      | P2   | Lc       | Le       |
| ---- | ---- | --------------------------------------- | ---- | -------- | -------- |
| `e0` | `03` | `00`: first transaction data block      | `00` | variable | variable |
|      |      | `80`: subsequent transaction data block |      |          |          |

##### Request payload

First data block:

| Description                                      | Length |
| ------------------------------------------------ | ------ |
| Number of BIP 32 derivations to perform (max 10) | 1      |
| First derivation index (big endian)              | 4      |
| ...                                              | 4      |
| Last derivation index (big endian)               | 4      |
| RLP data chunk                                   | var    |

Subsequent data blocks:

| Description    | Length |
| -------------- | ------ |
| RLP data chunk | var    |

#### **Response** format

| Description | Length |
| ----------- | ------ |
| v           | 1      |
| r           | 32     |
| s           | 32     |

#### Examples

**Command**: `e003000041058000002c800001f7800000000000000000000000eb1284561f61b9831e84809410109fc8df283027b6285cc889f5aa624eac1f55843b9aca0081800182040580`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                                                                                                                       |
| ------ | ------ | ------ | ------ | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0xe0` | `0x03` | `0x00` | `0x00` | `0x41` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000 0xeb1284561f61b9831e84809410109fc8df283027b6285cc889f5aa624eac1f55843b9aca0081800182040580` |

`44'/503'/0'/0/0` is encoded as `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000`.

`0xeb1284561f61b9831e84809410109fc8df283027b6285cc889f5aa624eac1f55843b9aca0081800182040580` is the RLP encoded list `["0x12", "0x561f61b9", "0x1e8480", "0x10109fC8DF283027b6285cc889F5aA624EaC1F55", "0x3b9aca00", "0x80", "0x1", "0x405", "0x"]` which represents the following transaction:

```js
{
    nonce: 18,
    gasPrice: 1444897209,
    gasLimit: 2000000,
    to: '0x10109fC8DF283027b6285cc889F5aA624EaC1F55',
    value: 1000000000,
    storageLimit: 128,
    epochHeight: 1,
    chainId: 1029,
    data: '0x
}
```

**Response**: `00 f9071161c2dbc19dabf54d14d42944cecacf61943a9898f4f64c8aa6d23a58b6 64ea364f092d23d7a94388f2f43cf54a86fe644d221e822210fde413d406ebb6 9000`

---

The same transaction sent in two chunks:

**Command**: `e00300002b058000002c800001f7800000000000000000000000eb1284561f61b9831e84809410109fc8df283027b628`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                                                                           |
| ------ | ------ | ------ | ------ | ------ | ------------------------------------------------------------------------------------------------------------ |
| `0xe0` | `0x03` | `0x00` | `0x00` | `0x2b` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000 0xeb1284561f61b9831e84809410109fc8df283027b628` |

**Response**: `9000`

**Command**: `e0038000165cc889f5aa624eac1f55843b9aca0081800182040580`

| CLA    | INS    | P1     | P2     | Lc     | Le                                               |
| ------ | ------ | ------ | ------ | ------ | ------------------------------------------------ |
| `0xe0` | `0x03` | `0x80` | `0x00` | `0x16` | `0x5cc889f5aa624eac1f55843b9aca0081800182040580` |

**Response**: `00 f9071161c2dbc19dabf54d14d42944cecacf61943a9898f4f64c8aa6d23a58b6 64ea364f092d23d7a94388f2f43cf54a86fe644d221e822210fde413d406ebb6 9000`

## SIGN_PERSONAL

#### Request format

| CLA  | INS  | P1                          | P2   | Lc       | Le       |
| ---- | ---- | --------------------------- | ---- | -------- | -------- |
| `e0` | `04` | `00`: first data block      | `00` | variable | variable |
|      |      | `80`: subsequent data block |      |          |          |

##### Request payload

First data block:

| Description                                      | Length |
| ------------------------------------------------ | ------ |
| Number of BIP 32 derivations to perform (max 10) | 1      |
| First derivation index (big endian)              | 4      |
| ...                                              | 4      |
| Last derivation index (big endian)               | 4      |
| Chain ID                                         | 4      |
| Message length                                   | 4      |
| Message chunk                                    | var    |

Subsequent data blocks:

| Description   | Length |
| ------------- | ------ |
| Message chunk | var    |

#### **Response** format

| Description | Length |
| ----------- | ------ |
| v           | 1      |
| r           | 32     |
| s           | 32     |

#### Examples

**Command**: `e00400002a058000002c800001f7800000000000000000000000000004050000000d48656c6c6f2c20776f726c6421`

| CLA    | INS    | P1     | P2     | Lc     | Le                                                                                                               |
| ------ | ------ | ------ | ------ | ------ | ---------------------------------------------------------------------------------------------------------------- |
| `0xe0` | `0x04` | `0x00` | `0x00` | `0x2a` | `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000 0x00000405 0x0000000d 0x48656c6c6f2c20776f726c6421` |

`44'/503'/0'/0/0` is encoded as `0x05 0x8000002c 0x800001f7 0x80000000 0x00000000 0x00000000`.

`0x00000405` stands for chain ID 1029 (Conflux mainnet).

`0x0000000d` is the length of the subsequent message (13 bytes).

`0x48656c6c6f2c20776f726c6421` is the message `"Hello, world!"` hex-encoded.

**Response**: `00 07954c638fc7de7cdc26c69633ad0202f8a20842b49508baa3e63166961b517a 70b2e651babb1566ae53f8c1ff40c8b5426366e0d5d2de2b0fae9ed2209de53e 9000`
