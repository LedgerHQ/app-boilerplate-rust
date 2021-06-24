#!/usr/bin/env python3
# coding: utf-8
"""
@BANNER@
"""
# -----------------------------------------------------------------------------
import sys
import os
from PIL import Image


# -----------------------------------------------------------------------------
class GIF2RS (object):
    """
    This class will contain methods to handle GIF bitmaps & generate .rs files
    """

# -------------------------------------------------------------------------
    def __init__(self, name):
        super().__init__()
        # Store parameters:
        self.name = name
        try:
            # Load the .GIF picture:
            sys.stdout.write(f"Loading {self.name}\n")
            self.img = Image.open(self.name)
            # Be sure it is a B&W picture:
            if False and self.img.mode not in ("1", "L"):
                sys.stderr.write(f"{self.name} is not a B&W picture!\n")
                sys.exit(-3)

        except (BaseException, Exception) as error:
            sys.stderr.write(f"Error with file {self.name}: {error}\n")
            sys.exit(-1)

    # -------------------------------------------------------------------------
    def __enter__(self):
        """
        Return an instance to this object.
        """
        return self

    # -------------------------------------------------------------------------
    def __exit__(self, exec_type, exec_value, traceback):
        """
        Do all the necessary cleanup.
        """

    # -------------------------------------------------------------------------
    # (based on icon3.py source code)
    @staticmethod
    def image_to_packed_buffer(img):
        """
        Rotate and pack bitmap data of the character.
        """
        width, height = img.size

        current_byte = 0
        current_bit = 0
        image_data = []

        # Row first
        for row in range(height):
            for col in range(width):
                # Return an index in the indexed colors list
                # left to right
                # Perform implicit rotation here (0,0) is left top in BAGL,
                # and generally left bottom for various canvas
                color_index = img.getpixel((col, row))
                # Simplification:if color is not 0 (black) then it is 1 (white)!
                if color_index >= 1:
                    color_index = 1
                else:
                    color_index = 0

                # le encoded
                current_byte += color_index << current_bit
                current_bit += 1

                if current_bit >= 8:
                    image_data.append(current_byte & 0xFF)
                    current_bit = 0
                    current_byte = 0

        # Handle last byte if any
        if current_bit > 0:
            image_data.append(current_byte & 0xFF)

        return bytes(image_data)


# -----------------------------------------------------------------------------
# Program entry point:
# -----------------------------------------------------------------------------
if __name__ == "__main__":

    # -------------------------------------------------------------------------
    def main(argc, argv):

        # Check parameters:
        if argc != 2:
            sys.stderr.write(f"Syntax: {argv[0]} filename\n")
            return 1

        # Load the provided picture (more formats than just GIF are supported):
        with GIF2RS(argv[1]) as gif:

            width, height = gif.img.size

            # Last step: generate .rs file with bitmap data:
            filename = os.path.splitext(os.path.basename(gif.name))[0]
            filename += ".rs"
            sys.stdout.write(f"Generating file {filename} ({width}x{height})\n")
            with open(filename, 'w') as txt:
                txt.write(f"let width: u32 = {width};\n")
                txt.write(f"let height: u32 = {height};\n\n")
                # Write the array containing all bitmaps:
                image_data = gif.image_to_packed_buffer(gif.img)

                txt.write(f"let bitmap = [\n")
                for i in range(0, len(image_data), 8):
                    txt.write("  " + ", ".join("0x{0:02X}".format(c) for c
                                               in image_data[i:i+8]) + ",")
                    txt.write("\n")
                txt.write("];\n")

        return 0

    # -------------------------------------------------------------------------
    # Call main function:
    exit_code = main(len(sys.argv), sys.argv)

    sys.exit(exit_code)
