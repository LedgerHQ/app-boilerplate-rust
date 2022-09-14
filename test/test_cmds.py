from ledgerblue.commTCP import getDongle as getDongleTCP
from ledgerblue.comm import getDongle

from random import getrandbits as rnd
from binascii import hexlify, unhexlify

rand_msg = hexlify(rnd(256).to_bytes(32, 'big')).decode()

CMDS = [
    "8002",
    "8003000020" + "00112233445566778899aabbccddeeff0123456789abcdeffedcba9876543210",
    "8003000020" + rand_msg,
    "8004",
    "80050008",
    "80FE",
    "80FF",
]

d = getDongleTCP(port=9999)     # Speculos
# d = getDongle()               # Nano

from time import sleep
for cmd in map(unhexlify,CMDS):
    r = None 
    try:
        r = d.exchange(cmd, 20)
        sleep(1)
    except Exception as e:
        print(e)
    if r is not None: 
        print("Response : ", hexlify(r))
