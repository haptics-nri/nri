#! /bin/make
##############################################
# Makefile for the 'wxPropGrid' module,
##############################################
#-----------------------------------------
DEST:=$(shell basename `pwd`)

#-----------------------------------------
# needed by dynamic libs
#-----------------------------------------
MAJOR=$(GPV_MAJOR)
MINOR=$(GPV_MINOR)
BUILD=$(GPV_BUILD)

#-----------------------------------------
MODULE_NAME=wxPropGrid
LIB_NAME=lib$(MODULE_NAME)

#-----------------------------------------
ROOT_PATH=../..
WORK=$(ROOT_PATH)/..
SYSTEM_INC_PATH=$(WORK)/common/System
SYSTEM_PATH=$(SYSTEM_INC_PATH)/linux

#-----------------------------------------
vpath %.cpp ..

#-----------------------------------------
SOURCES=                                 \
    advprops.cpp                         \
    odcombo.cpp                          \
    propgrid.cpp                         \
    props.cpp

#-----------------------------------------
OBJECTS=                                 \
    advprops.o                           \
    odcombo.o                            \
    propgrid.o                           \
    props.o

#-----------------------------------------
HEADERS=*.h

#-----------------------------------------
# You may need to adapt this path for your system.
# If $WX_CONF_TOOL is set by the environment then it is just used.
# Otherwise, by default we use "which" except on a mvBL or if a special purpose-built wxWidgets exists.
#-----------------------------------------
ifeq ($(strip $(WX_CONF_TOOL)),)
    ifeq ($(DEST),ppc603e)
        WX_CONF_PATH=$(GCCPATH)/bin/
        WX_CONF_TOOL=$(shell if [ -x $(WX_CONF_PATH)wx-config ]; then echo $(WX_CONF_PATH)wx-config; fi )
        ADD_LIBS=                        \
            -lz -lX11 -lXext -lpng -ljpeg
    else ifeq ($(DEST),armv7a) # mvBL-X
        ifneq ($(OE_HOME),)
        # if OE_HOME is defined, use OE tools
            WX_CONF_TOOL=$(OE_HOME)/tmp/sysroots/$(shell uname -m)-linux/usr/armv7a/bin/wx-config
        else
            BLX_SDK=$(SDK_PATH)
            WX_CONF_TOOL=$(BLX_SDK)/arm-angstrom-linux-gnueabi/usr/bin/wx-config
        endif
    else
        # check for specially-built static wxWidgets
        WX_SPECIAL_CONF_TOOL:=$(shell ls /opt/wx/2.6/$(DEST)/bin/wx-config 2>/dev/null)
        ifneq ($(WX_SPECIAL_CONF_TOOL),)
            WX_CONF_TOOL:=$(WX_SPECIAL_CONF_TOOL)
        else
            # use the system's wxWidgets
            WX_CONF_TOOL:=$(shell which wx-config-2.8 2>/dev/null)
            ifeq ($(WX_CONF_TOOL),)
                WX_CONF_TOOL:=$(shell which wx-config-2.6 2>/dev/null)
                ifeq ($(WX_CONF_TOOL),)
                    WX_CONF_TOOL:=$(shell which wx-config 2>/dev/null)
                endif
            endif
        endif
        ADD_LIBS=
    endif
endif

#-----------------------------------------
WXRELEASE=$(shell $(WX_CONF_TOOL) --release)
WXLIBFLAGS=$(shell $(WX_CONF_TOOL)  --libs)
WXCXXFLAGS=$(shell $(WX_CONF_TOOL)  --cxxflags)

#-----------------------------------------
ifeq ($(strip $(PKG_CONF_TOOL)),)
    ifeq ($(DEST),armv7a) # mvBL-X
        ifneq ($(OE_HOME),)
            # if OE_HOME is defined, use OE tools
            PKG_CONF_TOOL=$(OE_HOME)/tmp/sysroots/$(shell uname -m)-linux/usr/bin/pkg-config
        else
            ifneq ($(SDK_PATH),)
                BLX_SDK=$(SDK_PATH)
                WX_CONF_TOOL=$(BLX_SDK)/bin/pkg-config
            else
                $(warning "Need to have OE_HOME or SDK_PATH defined to find pkg-config")
            endif
        endif
    else
        PKG_CONF_TOOL:=$(shell which pkg-config 2>/dev/null)
    endif
endif
ifneq ($(strip $(PKG_CONF_TOOL)),)
    GTK2LIBFLAGS=$(shell $(PKG_CONF_TOOL)  --libs gtk+-2.0)
    GTK2CXXFLAGS=$(shell $(PKG_CONF_TOOL)  --cflags gtk+-2.0)
endif
#-------------------------------------------
# Local typedefs warning since gcc version 4.8 (wxWidgets code)
GCC_MAJOR          := $(shell g++ -dumpversion 2>&1 | cut -d'.' -f1)
GCC_MINOR          := $(shell g++ -dumpversion 2>&1 | cut -d'.' -f2)
NOLOCALTYPEDEFWARNING := $(shell if [ $(GCC_MAJOR) -ge 4 ] && [ $(GCC_MINOR) -ge 8 ]; then		\
						 echo "-Wno-unused-local-typedefs";			\
						 else										\
						 echo "";									\
						 fi )
#-----------------------------------------
ifeq ($(DEST),ppc603e)
CPPFLAGS_1=                              \
    -pedantic -Wno-long-long             \
    $(WXCXXFLAGS)

# ToDo: HG 2.7.07: New wxPropGrid possibly needs gtk.h. We find it by using pkg-config to give us the correct include options
# This will fail on a mvBL. We need to find another way to build wxPropView on the mvBL

else
CPPFLAGS_1= -I../../Include              \
    $(NOLOCALTYPEDEFWARNING)             \
    $(WXCXXFLAGS)                        \
    $(GTK2CXXFLAGS)                      \
    -pedantic -Wno-long-long             \
    -Wno-variadic-macros                 \
    -Wno-strict-aliasing
endif

#-----------------------------------------
USR_LIBS=                                \
    $(WXLIBFLAGS)                        \
    $(GTK2LIBFLAGS)                      \

#-----------------------------------------
USE_LN_LIBS=                             \

USER_AR_LIB_LIST=$(USE_LN_LIBS)

#-----------------------------------------
ifeq (,$(WX_CONF_TOOL))
    DO_DYNAMIC_LIBS=n
else
    DO_DYNAMIC_LIBS=y
endif

#-----------------------------------------
CPP_SYM_VISIBLE=y
DO_STATIC_LIBS=n

##############################################
# compiler
# will be set in '$(SYSTEM_PATH)/Makefile.all.inc'
# set    $(CC)     - for *.c files
# or    $(CXX)    - for *.cpp files
#-----------------------------------------
CTYPE=$(CXX)

##############################################
#-----------------------------------------
include $(SYSTEM_PATH)/Makefile.mod.inc

#-----------------------------------------
