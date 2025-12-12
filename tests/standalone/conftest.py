from ragger.conftest import configuration
from ragger.navigator import NavInsID
import pytest
###########################
### CONFIGURATION START ###
###########################

# You can configure optional parameters by overriding the value of ragger.configuration.OPTIONAL_CONFIGURATION
# Please refer to ragger/conftest/configuration.py for their descriptions and accepted values

#########################
### CONFIGURATION END ###
#########################

# Pull all features from the base ragger conftest using the overridden configuration
pytest_plugins = ("ragger.conftest.base_conftest", )

# Notes :
# 1. Remove this fixture once the pending review screen is removed from the app
# 2. This fixture clears the pending review screen before each test
# 3. The scope should be the same as the one configured by BACKEND_SCOPE in 
# ragger/conftest/configuration.py
# @pytest.fixture(scope="class", autouse=True)
# def clear_pending_review(firmware, navigator):
#     # Press a button to clear the pending review
#     if firmware.device.startswith("nano"):
#         print("Clearing pending review")
#         instructions = [
#             NavInsID.BOTH_CLICK,
#         ]
#         navigator.navigate(instructions,screen_change_before_first_instruction=False)
