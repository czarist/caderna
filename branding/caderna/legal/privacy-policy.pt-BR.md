# Política de Privacidade do {{app}}

Versão {{version}} · Última atualização: 13 de novembro de 2026.

Esta Política explica como tratamos dados pessoais no aplicativo {{app}}, no painel da
escola (área do cliente) e no site do {{app}}, de acordo com a Lei nº 13.709/2018 (Lei Geral
de Proteção de Dados, LGPD). Ela faz parte dos Termos de Uso.

## 1. Resumo

- O {{app}} roda no computador do aluno. Os estudos (editais, planos, anotações, fotos,
  respostas e conversas) ficam nele.
- O Painel guarda o que é preciso para a licença, as turmas e a cobrança: computadores
  ativados, planos publicados pelos professores, o progresso de cada aluno nas turmas e a
  quantidade de IA usada.
- O conteúdo das tarefas de IA passa pelo Painel até o fornecedor de IA e **não fica
  guardado** no Painel.
- O áudio do microfone é transcrito no próprio computador e não é enviado a ninguém.
- Não há telemetria, anúncios, cookies de rastreio nem venda de dados.

## 2. Quem é responsável

- **{{provider}}** ("Caderna Educação", "nós") desenvolve o {{app}} e opera o Painel e o site.
- Somos **controladores** dos dados das Escolas clientes e de seus representantes, dos
  contatos feitos pelo site, da cobrança, da segurança e dos registros de acesso.
- Quanto aos professores e alunos que a Escola cadastra e ao progresso deles nas turmas, a
  **Escola é a controladora** e nós somos **operadores**: tratamos esses dados segundo as
  instruções da Escola e os Termos de Uso. A Escola responde por ter base legal para
  cadastrá-los e por atender os pedidos deles, com o nosso apoio.

## 3. Dados que tratamos

**No site**

- Formulário "Prefiro que me chamem": nome, escola, cargo, email, telefone, faixa de
  alunos, melhor período, canal preferido e mensagem.
- Mensagem por email: assunto, nome, escola, email e mensagem.
- Conversa pelo WhatsApp: o assunto escolhido; a conversa em si acontece no WhatsApp.
- Em todos os casos: data e hora, o IP guardado como resumo criptográfico (hash) e
  informações do navegador, para segurança e prova do aviso.

**Na área do cliente (Escolas)**

- Dados da Escola: nome, CPF ou CNPJ, e nome, email e telefone do responsável.
- Dados de quem acessa a área do cliente: nome, email e registros de acesso. O acesso usa
  um código de uso único enviado por email.
- Cobranças, pagamentos e franquia de IA. Os dados do cartão são tratados pela Asaas; não
  guardamos o número do cartão.

**Professores e alunos (cadastrados pela Escola)**

- Nome, email, turmas e a licença de cada um.
- Do professor: os planos que ele publica (título, prova, matérias, tópicos, programa da
  prova, apostilas e questões).
- Do aluno: o progresso nos planos das turmas (situação e agenda de revisão de cada tópico,
  resultados de quizzes, resumos de aulas, revisões, caderno de erros) e o tempo de estudo
  por dia.

**No computador do usuário (não enviados ao Painel, salvo o descrito na seção 4)**

Ficam na pasta de dados do aplicativo (`%LOCALAPPDATA%\{{data_dir}}` no Windows):

- nome (como o tutor chama você), idioma e preferências;
- a chave de licença, sem criptografia no banco local, e a data do último contato com o
  Painel; o token do computador, renovado a cada 7 dias, no cofre de credenciais do
  sistema;
- editais, provas, materiais, planos, progresso, quizzes, revisões, apostilas, destaques,
  notas, conversas das aulas, caderno de erros, fotos de respostas discursivas e as suas
  avaliações;
- a identidade visual da Escola recebida do Painel;
- se a Escola bloquear capturas de tela: o nome e o email do aluno, que aparecem em
  marca-d'água nas apostilas;
- a data do aceite destes documentos e um registro técnico (log) sem imagens, respostas
  nem chaves.

Com as capturas bloqueadas pela Escola, no macOS e no Linux o aplicativo confere, cerca de
uma vez por segundo, os nomes dos programas abertos, para encobrir o conteúdo durante
gravações de tela. Essa lista é lida no computador e não é guardada nem enviada.

## 4. O que sai do computador do usuário

| Para | O quê | Quando |
|---|---|---|
| Painel | Chave de licença; código do computador (identificador do sistema passado por hash); versão do app; sistema operacional; endereço IP | Ao ativar, ao abrir o app e a cada hora |
| Painel, que repassa ao fornecedor de IA | Trechos do programa da prova e dos materiais, tópicos, mensagens e respostas, o nome do aluno (para o tutor chamá-lo) e textos das apostilas | Em cada tarefa de IA |
| Painel, para o modelo de visão do fornecedor de IA | Fotos de respostas discursivas e páginas de PDFs escaneados | Só quando o usuário escolhe a leitura pela nuvem; por padrão, as fotos são lidas no computador |
| Painel | Progresso do aluno e planos publicados pelo professor, descritos na seção 3 | A cada hora e em "Sincronizar agora" |
| Painel, base comum | Planos, apostilas e questões feitos pela IA, sem dados pessoais | Antes e depois de cada material feito pela IA |
| Wikipédia e Wikimedia Commons | Termo de busca de uma imagem e o nome do arquivo, para o crédito | Se as imagens estiverem ligadas |
| GitHub, Hugging Face e sites das vozes | Pedidos de download, que mostram o IP e a versão do app | Ao procurar atualizações e baixar modelos |
| Serviço de voz na nuvem | O texto que o tutor vai falar | Só se o usuário ativar a voz na nuvem |

O nome do computador não é enviado. O Painel guarda, por computador, o hash, o sistema, a
versão e as datas de ativação e do último contato; por tarefa de IA, só o modelo, a
quantidade de texto processado (tokens), o resultado e a duração.

## 5. Para que usamos e com qual base legal

| Finalidade | Base legal (LGPD) |
|---|---|
| Responder a contatos feitos pelo site | Procedimentos preliminares a contrato, a pedido do titular (art. 7º, V) |
| Ativar licenças, controlar vagas, entregar a IA e a identidade visual da Escola | Execução de contrato (art. 7º, V) |
| Turmas: entregar os planos dos professores, guardar e mostrar o progresso dos alunos | Execução do contrato com a Escola, por conta dela (art. 7º, V) |
| Marca-d'água com nome e email e bloqueio de capturas, quando a Escola os ativa | Legítimo interesse da Escola em proteger o seu material (art. 7º, IX), com aviso ao aluno nesta Política e no aplicativo |
| Cobrança, notas fiscais e registros contábeis | Execução de contrato e cumprimento de obrigação legal (art. 7º, II e V) |
| Registros de acesso ao Painel e ao site | Cumprimento de obrigação legal (art. 15 do Marco Civil da Internet) |
| Segurança, prevenção a fraudes e antispam | Legítimo interesse (art. 7º, IX) |
| Defesa em processos | Exercício regular de direitos (art. 7º, VI) |

Não usamos dados pessoais para publicidade, não vendemos dados e não tomamos decisões
automatizadas que afetem os direitos de alguém: a IA ajuda no estudo, e as notas de
discursivas são estimativas sem efeito oficial.

## 6. Com quem compartilhamos

- **Fornecedor de IA:** hoje, a Z.ai (Zhipu AI), que recebe só o conteúdo de cada tarefa
  de IA, pelo Painel, sem a chave de licença e sem o email. Se trocarmos de fornecedor,
  atualizamos esta Política.
- **Asaas Gestão Financeira S.A.:** processa cobranças e pagamentos das Escolas.
- **Hospedagem e envio de emails:** provedores de infraestrutura em nuvem que guardam o
  Painel e enviam os emails de acesso e de cobrança, sob contrato e com acesso restrito.
- **Google (reCAPTCHA):** quando a verificação antispam está ativa no site, ela só é
  carregada se o visitante pedir, e o Google recebe dados técnicos do navegador.
- **A própria Escola:** vê os professores e alunos que cadastrou e o progresso deles. Os
  professores veem o nome e o resumo do progresso dos alunos das suas turmas.
- **Autoridades:** quando houver obrigação legal ou ordem judicial.

## 7. Transferência internacional

O fornecedor de IA e alguns provedores de infraestrutura tratam dados fora do Brasil. A
transferência é necessária para executar o contrato a pedido do usuário (art. 33, IX, da
LGPD) e se limita ao conteúdo de cada tarefa. Por isso, recomendamos não colocar nas
mensagens à IA dados sensíveis, como saúde, religião ou dados de outras pessoas.

## 8. Por quanto tempo guardamos

- **No computador:** até o usuário apagar. "Desconectar este computador" apaga a chave e o
  token; apagar a pasta de dados remove tudo.
- **Contatos do site:** até 12 meses após o último contato, se não virarem contrato.
- **Progresso e cadastro de alunos e professores:** enquanto estiverem cadastrados pela
  Escola. Quando a Escola remove o aluno ou o curso, o progresso nele é apagado; ao fim do
  contrato, apagamos os cadastros e o progresso em até 90 dias.
- **Computadores ativados:** enquanto a licença existir e por até 6 meses depois.
- **Registros de acesso:** 6 meses (art. 15 do Marco Civil da Internet).
- **Cobranças e uso de IA:** 5 anos, por obrigação fiscal e contábil.
- **Base comum:** até removermos o material; ela não contém dados pessoais.

Vencidos os prazos, os dados são apagados ou anonimizados.

## 9. Segurança

Usamos conexão criptografada (HTTPS), acesso à área do cliente por código de uso único,
separação dos dados de cada Escola, tokens de computador com validade de 7 dias e acesso
interno restrito. No computador, os dados ficam protegidos pela conta de usuário do
sistema; proteja-o com senha. Em caso de incidente de segurança que possa causar risco ou
dano relevante, comunicamos a ANPD e os titulares afetados no prazo da regulamentação (art.
48 da LGPD) e avisamos as Escolas cujos dados foram atingidos.

## 10. Cookies

O site e o Painel usam só cookies necessários: sessão e proteção contra falsificação de
formulários (CSRF). Preferências de tela, como o tema claro ou escuro, ficam no próprio
navegador. Não usamos cookies de publicidade nem de estatística.

## 11. Seus direitos

A LGPD garante a confirmação do tratamento, o acesso, a correção, a anonimização, o
bloqueio ou a eliminação de dados desnecessários, a portabilidade, a informação sobre
compartilhamento, a revogação do consentimento e a oposição (art. 18).

- **No aplicativo:** você vê, corrige, exporta e apaga os seus dados diretamente.
- **Alunos e professores:** peçam à sua Escola ou a nós pelo contato {{contact}}; quando o
  pedido for sobre dados que a Escola controla, nós o encaminhamos a ela.
- **Demais pedidos:** pelo contato {{contact}}. Respondemos em até 15 dias.
- Você também pode reclamar à Autoridade Nacional de Proteção de Dados (ANPD).

## 12. Crianças e adolescentes

O {{app}} não se destina a crianças com menos de 12 anos. Dados de adolescentes de 12 a 17
anos são tratados no melhor interesse deles, a partir do cadastro feito pela Escola, que
informa os pais ou responsáveis e obtém deles as autorizações necessárias. Ao identificar
dados de uma criança tratados sem a autorização exigida, nós os apagamos.

## 13. Inteligência artificial

As respostas geradas pela IA podem conter erros, e a qualidade depende do modelo usado.
Veja os Termos de Uso, seção 8.

## 14. Encarregado pelo tratamento de dados

O encarregado (art. 41 da LGPD) é Lucas Cezar Trentin, que recebe reclamações, pedidos e
comunicações dos titulares e da ANPD pelo contato {{contact}}.

## 15. Alterações

Quando esta Política mudar, a nova versão terá uma nova data. Mudanças relevantes são
avisadas por email às Escolas com pelo menos 15 dias de antecedência, e o aplicativo pede um
novo aceite.

## 16. Contato

{{provider}}. Dúvidas sobre privacidade: {{contact}}.
