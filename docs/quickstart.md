# Quickstart

Crea `hello.n`:

```nC
use <io:output>;

hello {
    to main {
        println("Hello, nC");
    }

    fa:
        return true;
}
```

Comprueba, compila y ejecuta:

```bash
ncc check hello.n
ncc build hello.n
ncc run hello.n
```

La ruta normal es manual/raw y genera artefactos propios del fuente bajo
`.ncc/`: `.nm`, objeto, ejecutable y manifiesto. Para funciones con una entrada
nombrada usa `fn:` y `ref main`:

```nC
hello {
    fn:
        public:
            run() -> bool { return true; }

    ref main ->> run;

    fa:
        return true;
}
```

Formatea o migra con `nfmt hello.n --compact --in-place`.
