"""Python mirror of `dungeon fights SEED`: same generator, same monsters, same random numbers.

usage: python mirror.py SEED
"""
import sys


class Rng:
    def __init__(self, seed):
        self.state = abs(seed) % 2147483647
        if self.state == 0:
            self.state = 1

    def below(self, n):
        self.state = self.state * 48271 % 2147483647
        return self.state % n


class Entity:
    def __init__(self, name, hp, power):
        self.name, self.hp, self.max_hp, self.power = name, hp, hp, power

    def alive(self):
        return self.hp > 0

    def attack_power(self, rnd):
        return self.power

    def hurt(self, amount):
        self.hp = max(self.hp - amount, 0)


class Player(Entity):
    def __init__(self):
        super().__init__("hero", 30, 4)
        self.weapon_bonus = 3

    def attack_power(self, rnd):
        return super().attack_power(rnd) + self.weapon_bonus


class Goblin(Entity):
    def __init__(self):
        super().__init__("goblin", 12, 3)


class Troll(Entity):
    def __init__(self):
        super().__init__("troll", 22, 4)

    def attack_power(self, rnd):
        return super().attack_power(rnd) + (2 if self.hp * 2 < self.max_hp else 0)


class Dragon(Entity):
    def __init__(self):
        super().__init__("dragon", 40, 5)

    def attack_power(self, rnd):
        return super().attack_power(rnd) + (6 if rnd % 3 == 0 else 0)


def fight_rounds(player, monster, rng, armor, first_round):
    rnd = first_round
    while player.alive() and monster.alive():
        p_dmg = player.attack_power(rnd) + rng.below(4)
        monster.hurt(p_dmg)
        m_dmg = 0
        if monster.alive():
            m_dmg = max(monster.attack_power(rnd) + rng.below(3) - armor, 0)
            player.hurt(m_dmg)
        yield {"round": rnd, "p_dmg": p_dmg, "m_dmg": m_dmg, "p_hp": player.hp, "m_hp": monster.hp}
        rnd += 1


def main(seed):
    for kind, cls in (("goblin", Goblin), ("troll", Troll), ("dragon", Dragon)):
        rng = Rng(seed)
        hero = Player()
        monster = cls()
        print(f"-- {kind} (seed {seed})")
        for s in fight_rounds(hero, monster, rng, 0, 1):
            print(f"round {s['round']} hero {s['p_dmg']} {kind} {s['m_dmg']} hp {s['p_hp']}/{s['m_hp']}")
        print("hero wins" if hero.alive() else f"{kind} wins")


if __name__ == "__main__":
    main(int(sys.argv[1]))
