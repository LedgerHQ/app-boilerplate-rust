from application_client.boilerplate_command_sender import BoilerplateCommandSender
from application_client.boilerplate_response_unpacker import unpack_get_version_response, unpack_get_multifield_review_with_newline_response
from utils import ROOT_SCREENSHOT_PATH
from ragger.navigator import NavInsID

# Taken from the Cargo.toml, to update every time the version is bumped
MAJOR = 1
MINOR = 2
PATCH = 3 

# In this test we check the behavior of the device when asked to provide the app version
def test_version(backend):
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # Send the GET_VERSION instruction
    rapdu = client.get_version()
    # Use an helper to parse the response, assert the values
    assert unpack_get_version_response(rapdu.data) == (MAJOR, MINOR, PATCH)

# In this test we check the behavior of the device when asked to provide the app version

def test_multifield_review_with_newline(firmware, backend, navigator, test_name):
    client = BoilerplateCommandSender(backend)
    with client.get_multifield_review_with_newline():
        navigator.navigate_until_text_and_compare(NavInsID.RIGHT_CLICK,
                                                  [NavInsID.BOTH_CLICK],
                                                  "Approve",
                                                  ROOT_SCREENSHOT_PATH,
                                                  test_name)
    rapdu = client.get_async_response()
    assert unpack_get_multifield_review_with_newline_response(rapdu.data) == None
