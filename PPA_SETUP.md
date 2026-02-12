# PPA Setup Guide

Этот проект настроен для автоматической публикации deb пакетов в APT репозиторий на GitHub Pages.

## Как использовать PPA

### 1. Добавление репозитория

```bash
# Добавить GPG ключ
curl -fsSL https://orelsokolov.github.io/yaaa/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaaa-archive-keyring.gpg

# Добавить репозиторий
echo "deb [signed-by=/usr/share/keyrings/yaaa-archive-keyring.gpg] https://orelsokolov.github.io/yaaa/ ./" | sudo tee /etc/apt/sources.list.d/yaaa.list

# Обновить список пакетов
sudo apt update
```

### 2. Установка пакета

```bash
sudo apt install yaaa
```

### 3. Обновление пакета

```bash
sudo apt update
sudo apt upgrade yaaa
```

## Настройка PPA для разработчика

### Шаг 1: Создание GPG ключа

```bash
# Установить gnupg
sudo apt install gnupg

# Создать ключ
gpg --full-gen-key
```

Выберите:
- Тип ключа: **RSA and RSA** (1)
- Размер ключа: **4096**
- Срок действия: **0** (не истекает)
- Имя и email

### Шаг 2: Экспорт ключей

```bash
# Экспортировать приватный ключ (для GitHub Secrets)
gpg --export-secret-keys --armor YOUR_EMAIL@example.com > apt-signing-key.asc

# Экспортировать публичный ключ (также для GitHub Secrets)
gpg --export --armor YOUR_EMAIL@example.com > apt-signing-pubkey.asc
```

### Шаг 3: Добавление секретов в GitHub

Перейдите в настройки репозитория → Secrets and variables → Actions → New repository secret:

| Secret | Описание | Обязательно |
|--------|----------|-------------|
| `APT_SIGNING_KEY` | Содержимое файла `apt-signing-key.asc` | ✅ Да |
| `APT_SIGNING_PUBKEY` | Содержимое файла `apt-signing-pubkey.asc` | ✅ Да |
| `APT_SIGNING_KEY_PASSPHRASE` | Парольная фраза ключа | ❌ Только если устанавливали при создании ключа |

> **Примечание:** Если при создании GPG ключа вы оставили passphrase пустым (нажали Enter), то секрет `APT_SIGNING_KEY_PASSPHRASE` **не нужен**.

### Шаг 4: Включение GitHub Pages

1. Перейдите в Settings → Pages
2. Source: Deploy from a branch
3. Branch: `gh-pages` / `/(root)`
4. Save

### Шаг 5: Запуск workflow

Workflow запускается автоматически при создании тега `v*`, либо вручную через Actions → Build and Publish to PPA → Run workflow.

## Структура репозитория

После успешной публикации на GitHub Pages будет доступна следующая структура:

```
https://orelsokolov.github.io/yaaa/
├── KEY.gpg              # Публичный GPG ключ
├── InRelease            # Подписанный Release файл
├── Release              # Описание репозитория
├── Release.gpg          # Подпись Release
├── Packages             # Список пакетов
├── Packages.gz          # Сжатый список пакетов
└── yaaa_*.deb           # Файлы пакетов
```

## Удаление PPA

```bash
sudo rm /etc/apt/sources.list.d/yaaa.list
sudo rm /usr/share/keyrings/yaaa-archive-keyring.gpg
sudo apt update
```
