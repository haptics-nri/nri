LIBDIR = liboptoforce

CXXFLAGS = -ggdb -I$(LIBDIR)/src/lib/optoforce
LDFLAGS = -loptoforce

all: liboptoforce_adapter.so

liboptoforce_adapter.so: adapter.cpp liboptoforce/build/src/lib/optoforce/liboptoforce.so
	cp liboptoforce/build/src/lib/optoforce/liboptoforce.so .
	g++ -shared -fPIC -std=c++11 -o $@ $^ $(CXXFLAGS) $(LDFLAGS)

test_adapter: test_adapter.c liboptoforce_adapter.so
	gcc -o $@ $< $(CXXFLAGS) $(LDFLAGS) -L. -loptoforce_adapter -Wl,-rpath .

clean:
	rm liboptoforce_adapter.so test_adapter

