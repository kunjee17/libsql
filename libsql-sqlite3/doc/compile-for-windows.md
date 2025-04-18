# Notes On Compiling SQLite On Windows 11

Here are step-by-step instructions on how to build SQLite from
canonical source on a new Windows 11 PC, as of 2023-11-01:

  1. Install Microsoft Visual Studio. The free "community edition"
      will work fine.  Do a standard install for C++ development.
      SQLite only needs the
      "cl" compiler and the "nmake" build tool.

  2. Under the "Start" menu, find "All Apps" then go to "Visual Studio 20XX"
      and find "x64 Native Tools Command Prompt for VS 20XX".  Pin that
      application to your task bar, as you will use it a lot.  Bring up
      an instance of this command prompt and do all of the subsequent steps
      in that "x64 Native Tools" command prompt.  (Or use "x86" if you want
      a 32-bit build.)  The subsequent steps will not work in a vanilla
      DOS prompt.  Nor will they work in PowerShell.

  3. Install TCL development libraries.  This note assumes that you will
      install the TCL development libraries in the "`c:\Tcl`" directory.
      Make adjustments
      if you want TCL installed somewhere else.  SQLite needs both the
      "tclsh.exe" command-line tool as part of the build process, and
      the "tcl86.lib" library in order to run tests.  You will need
      TCL version 8.6 or later.

        1. Get the TCL source archive, perhaps from
      [https://www.tcl.tk/software/tcltk/download.html](https://www.tcl.tk/software/tcltk/download.html).
        2. Untar or unzip the source archive. CD into the "win/" subfolder
            of the source tree.
        3. Run: `nmake /f makefile.vc release`
        4. Run: `nmake /f makefile.vc INSTALLDIR=c:\Tcl install`
        5. CD to `c:\Tcl\lib`. In that subfolder make a copy of the
            "`tcl86t.lib`" file to the alternative name "`tcl86.lib`"
            (omitting the second 't').
        6. CD to `c:\Tcl\bin`. Make a copy of the "`tclsh86t.exe`"
            file into "`tclsh.exe`" (without the "86t") in the same directory.
        7. Add `c:\Tcl\bin` to your %PATH%. To do this, go to Settings
            and search for "path". Select "edit environment variables for
            your account" and modify your default PATH accordingly.
            You will need to close and reopen your command prompts after
            making this change.

  4. Download the SQLite source tree and unpack it. CD into the
      toplevel directory of the source tree.

  5. Set the TCLDIR environment variable to point to your TCL installation.
      Like this:
        - `set TCLDIR=c:\Tcl`

  6. Run the "`Makefile.msc`" makefile with an appropriate target.
      Examples:
        - `nmake /f makefile.msc`
        - `nmake /f makefile.msc sqlite3.c`
        - `nmake /f makefile.msc devtest`
        - `nmake /f makefile.msc releasetest`

## 32-bit Builds

Doing a 32-bit build is just like doing a 64-bit build with the
following minor changes:

1. Use the "x86 Native Tools Command Prompt" instead of "x64 Native Tools Command Prompt". "**x86**" instead of "**x64**".

2. Use a different installation directory for TCL.
   The recommended directory is `c:\tcl32`.
   Thus you end up with two TCL builds:
    - `c:\tcl` &larr;  64-bit (the default)
    - `c:\tcl32` &larr;  32-bit

3. Ensure that `c:\tcl32\bin` comes before `c:\tcl\bin` on
   your PATH environment variable.  You can achieve this using
   a command like:
    - `set PATH=c:\tcl32\bin;%PATH%`

## Building a DLL

The command the developers use for building the deliverable DLL on the
[download page](https://sqlite.org/download.html) is as follows:

~~~~cmd
nmake /f Makefile.msc sqlite3.dll USE_NATIVE_LIBPATHS=1 "OPTS=-DSQLITE_ENABLE_FTS3=1 -DSQLITE_ENABLE_FTS4=1 -DSQLITE_ENABLE_FTS5=1 -DSQLITE_ENABLE_RTREE=1 -DSQLITE_ENABLE_JSON1=1 -DSQLITE_ENABLE_GEOPOLY=1 -DSQLITE_ENABLE_SESSION=1 -DSQLITE_ENABLE_PREUPDATE_HOOK=1 -DSQLITE_ENABLE_SERIALIZE=1 -DSQLITE_ENABLE_MATH_FUNCTIONS=1"
~~~~

That command generates both the sqlite3.dll and sqlite3.def files.  The same
command works for both 32-bit and 64-bit builds.

## Statically Linking The TCL Library

Some utility programs associated with SQLite need to be linked
with TCL in order to function.  The [sqlite3_analyzer.exe program](https://sqlite.org/sqlanalyze.html)
is an example.  You can build as described above, and then
enter:

~~~~cmd
nmake /f Makefile.msc sqlite3_analyzer.exe
~~~~

And you will end up with a working executable.  However, that executable
will depend on having the "tcl86.dll" library somewhere on your %PATH%.
Use the following steps to build an executable that has the TCL library
statically linked so that it does not depend on separate DLL:

  1. Use the appropriate "Command Prompt" window - either x86 or
      x64, depending on whether you want a 32-bit or 64-bit executable.

  2. Untar the TCL source tarball into a fresh directory.  CD into
      the "win/" subfolder.

  3. Run: `nmake /f makefile.vc OPTS=nothreads,static shell`

  4. CD into the "Release*" subfolder that is created (note the
      wildcard - the full name of the directory might vary).  There
      you will find the "tcl86s.lib" file.  Copy this file into the
      same directory that you put the "tcl86.lib" on your initial
      installation.  (In this document, that directory is
      "`C:\Tcl32\lib`" for 32-bit builds and
      "`C:\Tcl\lib`" for 64-bit builds.)

  5. CD into your SQLite source code directory and build the desired
      utility program, but add the following extra arguments to the
      nmake command line:

      ~~~cmd
      CCOPTS="-DSTATIC_BUILD" LIBTCL="tcl86s.lib netapi32.lib user32.lib"
      ~~~

      <p>So, for example, to build a statically linked version of
      sqlite3_analyzer.exe, you might type:
      ~~~cmd
      nmake /f Makefile.msc CCOPTS="-DSTATIC_BUILD" LIBTCL="tcl86s.lib netapi32.lib user32.lib" sqlite3_analyzer.exe
      ~~~

  6. After your executable is built, you can verify that it does not
      depend on the TCL DLL by running:

      ~~~cmd
      dumpbin /dependents sqlite3_analyzer.exe
      ~~~
