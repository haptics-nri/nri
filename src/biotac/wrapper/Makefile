LIBS = cheetah.so biotac.so
GENS = $(LIBS:.so=.rs)

all: $(LIBS) $(GENS)

%.so: %.c %.h
	gcc -shared -fPIC -o $@ $<

%.rs: %.h
	rust-bindgen -o $@ $<

clean:
	rm $(LIBS) $(GENS)

