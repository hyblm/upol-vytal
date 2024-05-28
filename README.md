# Instalace build systému cargo

- Možné přeskočit, pokud už je na systému funkční rust prostředí

- Doporučená metoda instalace je pomocí nástroje `rustup` [zdroj v dokumentaci](https://doc.rust-lang.org/book/ch01-01-installation.html#installation)
    - pro systémy mac a linux příkaz níže, nebo pomocí package manageru
    ```
    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
    ```
    - pro windows je k diposici [instalační soubor](https://www.rust-lang.org/tools/install)

- Pro alternativní instalaci jsou k dispozici [archivy s binárními soubory](https://forge.rust-lang.org/infra/other-installation-methods.html#standalone-installers)


# Spuštění programu

```
cargo run --release <cesta k dimacs souboru>
```

# Experimentální vyhodnocení a dokumentace k implementaci

následující příkaz dokument vygeneruje a otevře v prohlížeči.
```
cargo doc --open --document-private-items
```
