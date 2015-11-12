#! /bin/make

SUB_DIRS=$(shell ls)

#-------------------------------------------------------------------------
.PHONY: all clean strip build new info linfo ppc603e ppc x86 x86_64 cygwin native ppc_6xx arm armv7a

#=========================================================================
all clean strip new build ppc603e ppc x86 x86_64 cygwin info linfo native ppc_6xx arm armv7a:
	@for sdir in $(SUB_DIRS) ;							\
	do													\
		echo $$sdir; 									\
		(												\
			if [ -d $$sdir ];							\
			then										\
				if [ -e $$sdir/Makefile ];				\
				then									\
					echo "make in :"$$sdir;				\
					$(MAKE) -C $$sdir $@ || exit $$?;	\
				fi;										\
			fi											\
		)												\
		done

#-------------------------------------------------------------------------
