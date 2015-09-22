OPENNIDIR = openni2

CXXFLAGS = -I$(OPENNIDIR)/Include
LDFLAGS = -LBin/x64-Release -Wl,-rpath ./ -lOpenNI2

all: openni2/Bin/x64-Release/libOpenNI2.so libOpenNI2_adapter.so

openni2/Bin/x64-Release/libOpenNI2.so: openni2/Makefile
	make -C openni2

libOpenNI2_adapter.so: adapter.cpp
	g++ -shared -fPIC -o $@ $^ $(CXXFLAGS) $(LDFLAGS)

clean:
	rm libOpenNI2_adapter.so

