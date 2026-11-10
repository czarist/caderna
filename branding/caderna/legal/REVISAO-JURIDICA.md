# Revisão jurídica dos textos do Caderna

Para: advogado(a) que revisará os Termos de Uso e a Política de Privacidade do Caderna, e
para a prestadora (Caderna Educação), que precisa tomar as decisões de negócio marcadas abaixo.

Textos em revisão (versão `2026-11-10.1`):

- `terms-of-use.pt-BR.md` (Termos de Uso)
- `privacy-policy.pt-BR.md` (Política de Privacidade)

As marcações `{{app}}`, `{{company}}`, `{{provider}}`, `{{licensor}}`, `{{contact}}`,
`{{data_dir}}` e `{{version}}` dos textos são preenchidas no build com os valores de
`branding/caderna/brand.toml`, seção `[legal]`. O app mostra os textos na primeira abertura
e pede um novo aceite sempre que a versão muda.

O Caderna é oferecido só no Brasil e só em português do Brasil. Não há versões em outros
idiomas ou regiões.

## Novidades da versão 2026-11-10.1 (turmas, nuvem e base comum)

- Professores e alunos cadastrados pela escola: o painel guarda os planos publicados e o
  progresso de cada aluno, visível aos professores da escola. Revisar a base legal
  (execução de contrato com a escola ou com o aluno), o papel da escola (controladora ou
  operadora quanto ao progresso dos alunos) e o consentimento de responsáveis para menores.
- Base comum entre escolas: materiais feitos pela IA (sem dados pessoais) ficam no painel e
  são reaproveitados por todas as escolas. Decidir se o contrato com as escolas precisa
  prever esse compartilhamento e quem responde por material errado.
- Referências bibliográficas nas apostilas, sugeridas e conferidas pela IA, com aviso nos
  Termos de que podem estar erradas.

## 1. O que o app faz com dados (fatos técnicos)

Estes pontos foram conferidos no código e num teste com o painel em 9/11/2026. Os textos
já os descrevem; a revisão é sobre forma, bases legais e lacunas.

| Dado | Onde fica | Quem recebe |
|---|---|---|
| Estudos (editais, planos, progresso, anotações, fotos, respostas, conversas) | Só no computador do aluno | Ninguém, exceto o trecho necessário a cada tarefa de IA |
| Áudio do microfone | Transcrito no computador | Ninguém |
| Chave de licença | Banco local, sem criptografia | Painel da prestadora, ao ativar e a cada verificação |
| Código do computador (identificador do sistema passado por hash com o nome do produto) | — | Painel; o identificador original não sai do computador |
| Versão do app e sistema (Windows, macOS, Linux) | — | Painel |
| Nome do computador | — | **Ninguém** (o app não envia) |
| Token do computador (vale 7 dias) | Cofre de credenciais do sistema | Painel, em cada chamada |
| Conteúdo das tarefas de IA (trechos, mensagens, nome do aluno, fotos se ele escolher) | — | Painel, que repassa ao fornecedor de IA e **não guarda o conteúdo** |
| Registros no painel | Banco do painel | Por computador: hash, sistema, versão, datas. Por tarefa de IA: modelo, tokens, resultado, duração |
| Identidade visual (nome, cores, links, imagens, textos) | Pasta de dados do aluno | Vem do painel; baixar as imagens mostra o IP do aluno ao endereço delas |

Funcionamento sem internet: até 14 dias desde o último contato com o painel; depois o app
para até reconectar, sem apagar estudos. A prestadora pode desconectar ou bloquear um
computador pelo painel.

## 2. Decisões e dados que faltam

Cada item precisa de uma resposta antes da distribuição. Onde o texto tem uma escolha
provisória, ela está indicada.

1. **Identificação da prestadora.** `provider` ainda diz "Caderna Educação — empresa
   demonstrativa". Faltam razão social, CNPJ e endereço. O foro dos Termos usa a sede da
   prestadora.
2. **Canal de contato.** `contact` aponta para `https://example.com/suporte`. É usado nos
   dois textos, inclusive como canal do encarregado.
3. **Encarregado (art. 41 da LGPD).** A Política diz que o encarregado atende pelo canal de
   contato. Falta indicar quem é, ou confirmar a dispensa aplicável a agente de pequeno
   porte (Resolução CD/ANPD nº 2/2022) e, nesse caso, ajustar o texto.
4. **Papéis da LGPD.** Escolha provisória: a prestadora é controladora dos dados do painel;
   a licenciadora (TRENTIN SOLUÇÕES EM SOFTWARE LTDA – ME) só desenvolve o software. Se a licenciadora hospedar ou
   operar o painel, ela é operadora e falta o contrato de tratamento (art. 39) entre as duas.
5. **Fornecedor de IA.** O texto não o nomeia. Faltam nome, país, retenção pelo fornecedor,
   uso ou não dos dados para treinar modelos, e o mecanismo da transferência internacional
   (art. 33; cláusulas-padrão da Resolução CD/ANPD nº 19/2024).
6. **Hospedagem do painel.** Provedor e país do servidor, e se há outra transferência
   internacional.
7. **Prazos de guarda no painel.** Hoje: "enquanto a licença existir" (computadores) e
   "pelo prazo definido pela prestadora" (uso de IA). Falta um prazo concreto. Avaliar se o
   painel é provedor de aplicação sujeito à guarda de registros de acesso por 6 meses
   (art. 15 do Marco Civil da Internet).
8. **Bases legais.** Escolha provisória: execução de contrato (art. 7º, V) para licença e
   IA. Confirmar, principalmente para o envio do nome do aluno à IA e para alunos menores.
9. **Crianças e adolescentes.** O texto pede conhecimento e consentimento dos responsáveis
   para menores de 18. Para crianças (até 12 anos) o art. 14 exige consentimento específico
   de um dos pais. Decidir idade mínima ou um fluxo de consentimento.
10. **Relação de consumo.** Os Termos não tratam de preço, pagamento, renovação,
    cancelamento nem direito de arrependimento (art. 49 do CDC) da licença vendida ao
    aluno. Decidir se isso fica nos Termos ou num contrato de venda à parte.
11. **Cota de IA e suspensão.** Os Termos dizem que as tarefas de IA param ao atingir a
    cota e que a prestadora pode desconectar ou bloquear computadores. Conferir a clareza
    exigida pelo CDC (art. 6º, III) e se as hipóteses de bloqueio precisam ser listadas.
12. **Limitação de responsabilidade.** Item 11 dos Termos: conferir compatibilidade com o
    CDC (art. 51, I).
13. **Incidentes de segurança.** Definir quem comunica a ANPD e os titulares (art. 48) e
    em que prazo. A Política só cita o arquivo SECURITY.md do projeto.
14. **Licença do software.** Os Termos dizem que o app é licenciado à prestadora pela
    licenciadora e que componentes abertos mantêm suas licenças. Conferir com o contrato
    entre as duas e com a lista de créditos do README (vozes, fontes, modelos).
15. **Chave de licença sem criptografia.** É uma escolha técnica: no Linux o cofre do
    sistema perde os dados ao reiniciar. A Política diz isso no item 7. Confirmar se basta.

## 3. Depois da revisão

1. Ajustar os textos e os valores de `[legal]` em `branding/caderna/brand.toml`.
2. Mudar `version` em `[legal]` sempre que o conteúdo mudar. O app pede novo aceite.
3. Marcar `reviewed = true`, e `commercial_reviewed = true` depois de conferidas as
   licenças de terceiros. Enquanto um deles for `false`, o empacotamento para venda
   (`scripts/package.ps1`) recusa a marca.
4. Os textos precisam continuar dizendo que a IA pode errar e que a qualidade depende do
   modelo usado. Um teste automático confere isso.
5. Qualquer dado novo enviado ao painel entra na Política antes de ir para produção.
