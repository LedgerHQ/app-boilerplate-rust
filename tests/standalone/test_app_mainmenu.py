from ragger.navigator import NavInsID


# In this test we check the behavior of the device main menu
def test_app_mainmenu(device, navigator, test_name, default_screenshot_path):
    # Navigate in the main menu
    if device.is_nano:
        instructions = [
            NavInsID.RIGHT_CLICK,
            NavInsID.RIGHT_CLICK,
            NavInsID.RIGHT_CLICK
        ]
    else:
        instructions = [
            NavInsID.USE_CASE_HOME_SETTINGS,
            NavInsID.USE_CASE_SUB_SETTINGS_NEXT,
            NavInsID.USE_CASE_SUB_SETTINGS_EXIT
        ]
    navigator.navigate_and_compare(default_screenshot_path, test_name, instructions,
                                   screen_change_before_first_instruction=False)
