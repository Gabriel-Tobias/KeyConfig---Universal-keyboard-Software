# KeyConfig

Remapeamento universal para teclados genéricos no Windows.

## Estrutura

| Pasta | O que é |
|---|---|
| `crates/keyconfig-core` | Modelo de dados: teclas físicas, perfis e leitura/escrita em disco. Não depende do Windows. |
| `crates/keyconfig-engine` | Motor (`keyconfig.exe`): fica na bandeja, intercepta o teclado com um hook global e aplica o perfil ativo. |
| `crates/keyconfig-config` | Configurador (`keyconfig-config.exe`, egui): perfis, teclado clicável e editor de ações. Só fica aberto enquanto você edita. |

Clique esquerdo no ícone da bandeja abre o configurador; clique direito abre o menu. Ao salvar, o configurador avisa o motor, que recarrega o perfil na hora. Os dois `.exe` precisam ficar na mesma pasta.

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
Depois de editar o arquivo, use **Recarregar perfil** no menu do ícone.

### Ações

| `type` | O que faz | Campos |
|---|---|---|
| `key` | Vira outra tecla | `to` |
| `disabled` | Não faz nada | — |
| `shortcut` | Combinação (ex.: Ctrl+C), mantida enquanto a tecla estiver pressionada | `keys` |
| `macro` | Sequência de passos, uma vez por toque | `steps` |
| `text` | Digita um texto (aceita acentos e `\n` para Enter) | `text` |
| `run` | Abre programa, arquivo, pasta ou site | `command`, `args` (opcional) |

Passos de macro: `{ "tap": "KeyA" }`, `{ "down": "ShiftLeft" }`, `{ "up": "ShiftLeft" }`, `{ "delay_ms": 100 }`, `{ "text": "olá" }`. Teclas apertadas com `down` e não soltas são liberadas no fim da macro.

```json
"mappings": {
  "F13": { "type": "shortcut", "keys": ["ControlLeft", "KeyC"] },
  "F14": { "type": "macro", "steps": [
    { "tap": "MetaLeft" }, { "delay_ms": 300 }, { "text": "calculadora" }, { "delay_ms": 300 }, { "tap": "Enter" }
  ] },
  "F15": { "type": "text", "text": "Atenciosamente,\nGabriel" },
  "F16": { "type": "run", "command": "https://github.com" }
}
```

## Programas rodando como administrador

O Windows impede que um programa comum altere o teclado de janelas elevadas. Quando isso acontece, o KeyConfig mostra um aviso. Para funcionar em tudo:

1. Clique com o botão direito no ícone e escolha **Executar como administrador**.
2. No novo menu, ative **Iniciar com o Windows (como administrador)**. Isso cria uma tarefa agendada que abre o KeyConfig elevado no logon, sem pedir confirmação do UAC.
