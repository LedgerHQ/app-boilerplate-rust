from pathlib import Path

import tomli
from application_client.boilerplate_command_sender import BoilerplateCommandSender
from application_client.boilerplate_response_unpacker import unpack_get_app_and_version_response


# Test a specific APDU asking BOLOS (and not the app) the name and version of the current app
def test_get_app_and_version(backend):
    with open(Path(__file__).parent.parent.parent / "Cargo.toml", "rb") as f:
        package = tomli.load(f)["package"]
    # Use the app interface instead of raw interface
    client = BoilerplateCommandSender(backend)
    # Send the special instruction to BOLOS
    response = client.get_app_and_version()
    # Use an helper to parse the response, assert the values
    app_name, version = unpack_get_app_and_version_response(response.data)

    assert app_name == package["metadata"]["ledger"]["name"]
    assert version == package["version"]
