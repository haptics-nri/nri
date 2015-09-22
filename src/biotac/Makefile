LIBS = cheetah.so biotac.so

all: $(LIBS)

%.so: %.c %.h
	gcc -shared -fPIC -o $@ $<

clean:
	rm $(LIBS)

