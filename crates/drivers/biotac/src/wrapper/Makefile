GENS = cheetah.rs biotac.rs
LIBS = $(addprefix lib, $(GENS:.rs=.so))

all: $(LIBS) $(GENS)

lib%.so: %.c %.h
	gcc -shared -fPIC -o $@ $<

%.rs: %.h
	rust-bindgen -o $@ $<

clean:
	rm $(LIBS) $(GENS)

