# Arquitetura

O projeto é organizado como um workspace monorepo.

## Módulos

- engine: motor de jogo que utilizando vulkan.
- vk: única porta de entrada do Vulkan no workspace. Reexporta da `ash` o que
  usamos e acrescenta o módulo `raii`, um wrapper fino que dá drop automático e
  `Deref` para o objeto da `ash`. O engine não depende da `ash` diretamente.

# Diretrizes

- Evite colocar comentários em tudo; no máximo documentação breve.
- Documentação e comentários devem escritos em inglês, e apenas onde necessário.
