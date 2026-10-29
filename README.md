# KeyConfig

Remapeamento universal para teclados genéricos no Windows.

## Estrutura

| Pasta | O que é |
|---|---|
| `crates/keyconfig-core` | Modelo de dados: teclas físicas, perfis e leitura/escrita em disco. Não depende do Windows. |
| `crates/keyconfig-engine` | Motor (`keyconfig.exe`): fica na bandeja, intercepta o teclado com um hook global e aplica o perfil ativo. |

O configurador (interface gráfica) virá depois. Ele só abre quando o usuário for editar os perfis.

## Compilar

```powershell
cargo test
cargo build --release   # gera target\release\keyconfig.exe
```

## Configuração

Fica em `%APPDATA%\KeyConfig\`, criada automaticamente na primeira execução:

```text
settings.json          { "active_profile": "default" }
profiles\default.json
```

Exemplo de perfil:

```json
{
  "name": "Padrão",
  "layout": "75%",
  "language": "pt-BR-abnt2",
  "mappings": {
    "CapsLock": { "type": "key", "to": "Escape" },
    "MetaLeft": { "type": "disabled" }
  }
}
```

- **Teclas** usam a posição física no padrão W3C (`KeyA`, `Digit1`, `CapsLock`, `IntlRo`...), então o mesmo perfil funciona com qualquer idioma. A lista completa está em [keycode.rs](crates/keyconfig-core/src/keycode.rs).
- **`layout`**: `60%`, `65%`, `75%`, `tkl`, `100%`.
- **`language`**: `pt-BR-abnt2`, `en-US`, `en-US-intl`.
- **Ações**: `key` (vira outra tecla) e `disabled` (não faz nada). Macros vêm depois.

Depois de editar o arquivo, use **Recarregar perfil** no menu do ícone.

## Programas rodando como administrador

O Windows impede que um programa comum altere o teclado de janelas elevadas. Quando isso acontece, o KeyConfig mostra um aviso. Para funcionar em tudo:

1. Clique com o botão direito no ícone e escolha **Executar como administrador**.
2. No novo menu, ative **Iniciar com o Windows (como administrador)**. Isso cria uma tarefa agendada que abre o KeyConfig elevado no logon, sem pedir confirmação do UAC.
