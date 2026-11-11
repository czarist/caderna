# Roteiro de teste do MVP — Caderna (app + painel)

Teste de ponta a ponta em desenvolvimento: painel (CADERNA-DASHBOARD) em Docker na
máquina e app Caderna compilado com `dev_url = http://localhost:8000`.

Anote cada passo como **OK**, **Falhou** (com o que apareceu) ou **Pulei** (com o motivo).
Quando falhar, guarde o log do app (`<pasta de dados>/logs`) e um print.

## 0. Preparar

### Painel

```powershell
cd C:\projetos\caderna-dashboard
docker compose up -d --build        # reconstrói a imagem com o código atual e migra o banco
docker compose exec app php artisan db:seed --class=DevTestSeeder
```

O seeder cria a **Escola Teste (dev)** e imprime os códigos de acesso
(`XXXX-XXXX-XXXX-XXXX`). Pode rodar de novo: não duplica nada, só imprime os códigos outra
vez. Não roda em produção.

| Onde | Login | Senha |
|---|---|---|
| <http://localhost:8000/admin> | `admin@caderna.test` (admin) | `caderna-teste` |
| <http://localhost:8000/admin> | `operador@caderna.test` (operador) | `caderna-teste` |
| <http://localhost:8000/cliente> | `escola@caderna.test` (cliente) | `caderna-teste` |

Os códigos de login do painel e os emails dos alunos (com o link do código) chegam no
Mailpit: <http://localhost:8025>.

| Código (tabela do seeder) | Para que serve |
|---|---|
| `aluno.completo@caderna.test` | aluno da Turma Teste, todos os módulos |
| `aluno.conteudo@caderna.test` | aluno só com "Criação de conteúdos" |
| `professora@caderna.test` | professora (cria e publica o Curso Teste) |
| `seed: licença completa` | dono, 3 computadores, todos os módulos |
| `seed: avaliação` | avaliação de 14 dias |
| `seed: só videoaula` | só "Videoaula" |

### App

```powershell
cd C:\projetos\caderna
$env:KG_BRAND_DIR = "branding/caderna"
$env:KG_DATA_DIR = "$env:TEMP\caderna-teste"   # pasta limpa para o teste
cargo run --release
```

Para começar do zero, apague a pasta de `KG_DATA_DIR`.

> **Trocar de conta no mesmo computador:** o token do aparelho fica no cofre do Windows
> com um nome só por marca, mesmo com `KG_DATA_DIR` diferente. Antes de usar outro código,
> faça **Configurações → Desconectar este computador** na conta atual.

## 1. Primeira abertura e licença

1. Abrir com a pasta limpa: aparece a configuração inicial (idioma pt-BR, nome,
   professor, termos). A escolha de IA **não** aparece (a IA vem do painel).
2. Os termos e a política mostram a versão `2026-11-10.1` e falam do painel, da nuvem
   de progresso e da base comum.
3. Sem licença o app fica na tela de licença. Digitar um código inválido
   (`AAAA-BBBB-CCCC-DDDD`): mensagem clara.
4. Digitar o código `seed: licença completa` em minúsculas e com espaços no lugar dos
   traços: ativa online e abre o app.
5. No painel (admin → Escola Teste → Licenças), a licença mostra 1 computador em uso.
   O botão **Código** mostra o código curto (com copiar) e a chave longa.
6. Fechar e abrir de novo: entra direto, sem pedir o código.
7. A chave longa (do botão **Código** no admin) também ativa, em outra pasta de teste.

## 2. Marca vinda do painel

1. Logo após ativar, o app mostra **Escola Teste** (nome, cores azuis) sem precisar
   reabrir. O título e o ícone da janela também mudam.
2. Com o app aberto, mudar no painel (admin → Escola Teste → Marca) o nome e a cor de
   destaque. Em até 1 hora (ou ao reabrir) o app mostra a mudança sozinho.
3. Enviar um ícone PNG quadrado (64–2048 px) e uma marca SVG: aparecem no app.
   Enviar um PNG não quadrado: o painel ou o app recusa, o ícone antigo continua.
4. Gerar uma apostila (seção 4): o rodapé e a marca-d'água usam a Escola Teste.
5. O pop-up de lembrete (bandeja → lembretes) usa a mesma marca.

## 3. Plano de estudo (conta do dono ou da professora)

1. Enviar um edital em PDF (um edital real de concurso ou o do ENEM).
2. Escolher o propósito (ENEM, vestibular, concurso, OAB, outro): o app pergunta sempre.
3. O plano sai com matérias, tópicos e semanas; dá para editar e apagar.
4. Criar o mesmo plano de novo: a segunda vez usa a base comum (mais rápido).

## 4. Estudar

1. **Videoaula:** abrir uma aula, ouvir a voz em pt-BR, ver o quadro, falar pelo
   microfone e receber resposta. Parar e retomar.
2. **Apostila:** gerar, abrir no leitor, exportar PDF. Tem referências no fim.
3. **Questionário:** responder, ver correção. Testar o formato Cebraspe (certo/errado).
4. **Revisões:** um tópico estudado entra na revisão programada.
5. **Simulado** e **resposta discursiva** (foto da folha; OCR local ou visão do painel).
6. **Desafios/metas** e **lembretes na bandeja**.
7. Uso de IA aparece no painel (admin → Escola Teste → Uso de IA).

## 5. Módulos por licença

1. Desconectar este computador; ativar `aluno.conteudo@caderna.test`: a Videoaula some do
   menu e do tour; abrir por atalho mostra "módulo fechado" com o link de contato.
2. Na área do cliente, mudar o plano desse aluno para incluir Videoaula: na próxima
   checagem (ou ao reabrir) o módulo aparece.
3. Ativar `seed: só videoaula`: só a parte de aulas fica aberta.

## 6. Turmas e cursos na nuvem

1. Ativar `professora@caderna.test`: aparece a tela de Turmas. Publicar um plano para a
   **Turma Teste**. Republicar: a versão sobe.
2. Ativar `aluno.completo@caderna.test`: o curso da turma chega como plano só de
   leitura; não dá para criar nem apagar planos ("seus planos vêm da sua turma").
3. Estudar um tópico como aluno. Na área do cliente → Turma Teste → Desempenho, o
   progresso aparece.
4. Restaurar em outro computador (ou pasta limpa após "Desconectar este computador"): o
   progresso volta.
5. Tirar o curso da turma no painel: some do app do aluno na próxima checagem.

## 7. Área do cliente (`/cliente`, `escola@caderna.test`)

1. Entrar com senha + código do Mailpit.
2. Matricular um aluno novo: o email chega no Mailpit, com o tema Caderna, **sem o
   código**, só com o botão "Ver meu código de acesso".
3. Abrir o link: a página mostra o código e o botão **Copiar código**; copiar e colar no
   app ativa. Mudar uma letra da URL: "Este link expirou", sem mostrar nada da licença.
4. Reenviar o email; mudar plano; tornar professor; remover (a licença é revogada, o app
   desse aluno trava na próxima checagem e o link mostra "licença cancelada").
5. Passar do limite de alunos (baixar o limite no admin para testar): mensagem clara.
6. Cobranças: o admin cria uma cobrança com título e descrição (caixa de texto com
   várias linhas); não há escolha de parcelamento no admin. O cliente vê o título e a
   descrição, e no cartão escolhe de 1x a 3x sem juros ou de 4x a 10x com juros
   (o total com juros aparece na opção). Pagar por Pix e por cartão em 10x no sandbox
   Asaas: a fatura da Asaas mostra o total com juros; o webhook marca como paga e o email
   de confirmação mostra o valor pago.

## 8. Painel admin (`/admin`)

1. Admin precisa de código por email; operador entra sem obrigatoriedade.
2. Clientes: validar, suspender (o app da escola recusa), reativar.
3. Licenças: emitir, revogar (o app trava com o motivo), bloquear um computador.
4. Base comum: listar e remover um item.

## 9. Fora do ar e casos de borda

1. Parar o painel (`docker compose stop app`) com o app aberto: o app continua; em
   Configurações aparece "offline", com os dias restantes (14 de tolerância).
2. Ligar o painel de novo: volta a "conectado" sozinho.
3. Licença vencida (mudar a data no admin): o app para, os dados de estudo ficam.
4. Atualizador: Configurações → Sobre e atualizações (sem release nova: diz que está em
   dia).

## 10. Instalador (antes do lançamento)

1. `scripts/package.ps1 -Brand caderna` gera `Caderna-setup.exe`.
2. Instalar numa máquina limpa (sem Rust); abrir, ativar, ter voz e OCR baixados.
3. Desinstalar: confira o que acontece com a pasta de dados e se bate com os termos.
