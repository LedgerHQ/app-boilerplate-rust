MEMORY
{
  DISCARD (rwx) : ORIGIN = 0xd0000000, LENGTH = 1M
  FLASH   (rx)  : ORIGIN = 0xc0d00000, LENGTH = 400K
  SRAM    (rwx) : ORIGIN = 0x20001800, LENGTH = 4K
}

PAGE_SIZE  = 64;
STACK_SIZE = 2048;
END_STACK  = ORIGIN(SRAM) + LENGTH(SRAM);

SECTIONS
{
  .text :
  {
    /* provide start code symbol, shall be zero */
    _text = .;
    PROVIDE(_nvram = .);

    PROVIDE(_setjmp = setjmp); /*thanks clang*/

     /* ensure main is always @ 0xC0D00000 */
     *(.boot*)
    
    /* place the other code and rodata defined BUT nvram variables that are displaced in a r/w area */
    *(.text*)
    *(.rodata)
    *(.rodata.[^N]*) /*.data.rel.ro* not here to detect invalid PIC usage */
    *(.rodata.N[^_]*)

    . = ALIGN(4);
    
    /* all code placed */
    _etext = .;

    . = ALIGN(PAGE_SIZE);

    _nvram_data = .;
    
    /* NVM data (ex-filesystem) */
    *(.bss.N_* .rodata.N_*)

    . = ALIGN(PAGE_SIZE);
    _envram_data = .;

    _install_parameters = .;
    PROVIDE(N_install_parameters = .);
    PROVIDE(_envram = .);
    _nvram_data_size = _envram_data - _nvram_data;
    
  } > FLASH = 0x00

  .data (INFO):
  {
    . = ALIGN(4);

    _data = .;

    *(vtable)
    *(.data*)

    _edata = .;

  } > SRAM

  ASSERT( (_edata - _data) <= 0, ".data section must be empty" )

  .bss :
  {
    _bss = .;
    *(.bss*)
    _ebss = .;

    . = ALIGN(4);
    app_stack_canary = .;
    PROVIDE(app_stack_canary = .);
    . += 4;
    _stack_validation = .;
    . = _stack_validation + STACK_SIZE;
    _stack = ABSOLUTE(END_STACK) - STACK_SIZE;
    PROVIDE( _stack = ABSOLUTE(END_STACK) - STACK_SIZE);
    _estack = ABSOLUTE(END_STACK);
    PROVIDE( _estack = ABSOLUTE(END_STACK) );

  } > SRAM = 0x00

  /****************************************************************/
  /* DEBUG                                                        */
  /****************************************************************/

  .stack_sizes (INFO):
  {
    KEEP(*(.stack_sizes));
  }

  /* remove the debugging information from the standard libraries */
  DEBUG (INFO) :
  {
    libc.a ( * )
    libm.a ( * )
    libgcc.a ( * )
    *(.ARM.exidx* .gnu.linkonce.armexidx.*)
  } > DISCARD

}
