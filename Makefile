# Shaderlock Installation/Deployment Configuration

CARGO=cargo
FLAGS=--release

DEST=$(HOME)/.config/dynlock
PREFIX=/usr

notice:
	@echo "run 'make install'"

clean:
	rm dynlock.1
	${CARGO} clean

build:
	${CARGO} build ${FLAGS}

install: build
	mkdir -p ${DEST}
	mkdir -p ${PREFIX}/local/share/man/man1/
	cp -fr shaders ${DEST}/.
	cp -f default-config.yaml ${DEST}/config.yaml
	sudo install target/release/dynlock ${PREFIX}/bin/.
	sudo cp dynlock.1 ${PREFIX}/local/share/man/man1/

uninstall:
	rm -rf ${DEST}
	sudo rm -f ${PREFIX}/local/bin/dynlock
	sudo rm -f ${PREFIX}/local/share/man/man1/dynlock.1
