# from application_client.command_sender import ConfluxCommandSender
# from application_client.response_unpacker import unpack_get_app_and_version_response


# # Test a specific APDU asking BOLOS (and not the app) the name and version of the current app
# def test_get_app_and_version(backend, backend_name):
#     # Use the app interface instead of raw interface
#     client = ConfluxCommandSender(backend)
#     # Send the special instruction to BOLOS
#     response = client.get_app_and_version()
#     # Use an helper to parse the response, assert the values
#     app_name, version = unpack_get_app_and_version_response(response.data)

#     assert app_name == "app-conflux-rust"
#     assert version == "1.0.0"
