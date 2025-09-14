from pathlib import Path
import re

p = Path("src/main.rs")
s = p.read_text()

# a) Ensure INS mapping exists in the TryFrom<ApduHeader> (harmless if already there)
if "(INS_ECHO_HASH, 0, 0) => Ok(Instruction::EchoHash)" not in s:
    s = re.sub(
        r'\(\s*6\s*,[^)]*\)\s*=>\s*Ok\(Instruction::SignTx[^\)]*\)\s*,',
        r'\g<0>\n            (INS_ECHO_HASH, 0, 0) => Ok(Instruction::EchoHash),',
        s
    )

# b) Remove ANY existing EchoHash arms in handle_apdu
s = re.sub(r'\s*Instruction::EchoHash\s*=>\s*\{.*?\},', '', s, flags=re.S)

# c) Insert our single, borrow-safe EchoHash arm right after the SignTx arm in handle_apdu
echo_arm = r'''
        Instruction::EchoHash => {
            // Copy APDU body to a local stack buffer so the immutable borrow ends
            let buf = {
                let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
                if data.len() != 32 { return Err(AppSW::WrongApduLength); }
                let mut tmp = [0u8; 32];
                tmp.copy_from_slice(data);
                tmp
            };
            // Now we can mutably borrow comm
            comm.append(&buf);
            Ok(())
        },
'''

# Find the SignTx arm and insert after it
s = re.sub(
    r'(Instruction::SignTx\s*\{\s*chunk\s*:\s*\w+\s*,\s*more\s*:\s*\w+\s*\}\s*=>\s*handler_sign_tx\([^\)]*\)\s*,)',
    r'\1' + echo_arm,
    s,
    count=1,
    flags=re.S
)

p.write_text(s)
print("Patched src/main.rs")
