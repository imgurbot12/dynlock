# Shaderlock Installation/Deployment Configuration

CARGO=cargo
FLAGS=--release

DEST=$(HOME)/.config/dynlock

notice:
	@echo "run 'make install'"

clean:
	${CARGO} clean

build:
	${CARGO} build ${FLAGS}

install: build
	mkdir -p ${DEST}
	cp -fr shaders ${DEST}/.
	cp -f default-config.yaml ${DEST}/config.yaml
	install target/release/dynlock /usr/local/bin/.

uninstall:
	rm -rf ${DEST}
	rm -f /usr/local/bin/dynlock
