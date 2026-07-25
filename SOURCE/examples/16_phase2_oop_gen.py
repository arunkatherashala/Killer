import math
import random

# Helper functions for Killer language
def _typeof(value):
    if isinstance(value, bool):
        return "boolean"
    elif isinstance(value, int) or isinstance(value, float):
        return "number"
    elif isinstance(value, str):
        return "string"
    elif isinstance(value, list):
        return "array"
    elif isinstance(value, dict):
        return "object"
    elif callable(value):
        return "function"
    elif value is None:
        return "null"
    return "object"

# Math object
class Math:
    @staticmethod
    def sqrt(x):
        return math.sqrt(x)
    @staticmethod
    def abs(x):
        return abs(x)
    @staticmethod
    def pow(x, y):
        return x ** y
    @staticmethod
    def floor(x):
        return math.floor(x)
    @staticmethod
    def ceil(x):
        return math.ceil(x)
    @staticmethod
    def round(x):
        return round(x)
    @staticmethod
    def min(*args):
        return min(args) if args else float('inf')
    @staticmethod
    def max(*args):
        return max(args) if args else float('-inf')
    @staticmethod
    def random():
        return random.random()
    PI = math.pi
    E = math.e

# Array wrapper class to support Killer array methods
class KillerArray(list):
    def map(self, callback):
        return KillerArray([callback(item) for item in self])
    def filter(self, callback):
        return KillerArray([item for item in self if callback(item)])
    def reduce(self, callback, initial=None):
        if initial is None and len(self) == 0:
            raise ValueError('Reduce of empty array with no initial value')
        if initial is None:
            acc = self[0]
            start = 1
        else:
            acc = initial
            start = 0
        for i in range(start, len(self)):
            acc = callback(acc, self[i])
        return acc
    def find(self, callback):
        for item in self:
            if callback(item):
                return item
        return None
    def some(self, callback):
        for item in self:
            if callback(item):
                return True
        return False
    def every(self, callback):
        for item in self:
            if not callback(item):
                return False
        return True
    def sort(self, callback=None):
        if callback is None:
            sorted_arr = sorted(self)
        else:
            sorted_arr = sorted(self, key=lambda x: x, cmp=callback)
        self.clear()
        self.extend(sorted_arr)
        return self
    def reverse(self):
        list.reverse(self)
        return self
    def splice(self, start, deleteCount=None, *items):
        if deleteCount is None:
            deleteCount = len(self) - start
        deleted = self[start:start+deleteCount]
        self[start:start+deleteCount] = items
        return KillerArray(deleted)
    def join(self, sep=''):
        return sep.join(str(item) for item in self)

# Array object with static methods
class Array:
    @staticmethod
    def isArray(value):
        return isinstance(value, (list, KillerArray))

# Global functions
def parseInt(value, radix=10):
    if isinstance(value, int):
        return value
    s = str(value).strip()
    match = ''
    for c in s:
        if c.isdigit() or (c in '+-' and not match):
            match += c
        else:
            break
    return int(match) if match and match not in '+-' else 0

def parseFloat(value):
    if isinstance(value, (int, float)):
        return float(value)
    s = str(value).strip()
    match = ''
    for c in s:
        if c.isdigit() or c in '.-+':
            match += c
        else:
            break
    try:
        return float(match) if match else float(value)
    except:
        return 0.0

def String(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "null"
    return str(value)

def Number(value):
    if isinstance(value, (int, float)):
        return value
    if isinstance(value, bool):
        return 1 if value else 0
    if value is None:
        return 0
    if isinstance(value, str):
        if value == '':
            return 0
        try:
            return int(value) if '.' not in value else float(value)
        except:
            return float('nan')
    return float('nan')

def Boolean(value):
    if isinstance(value, bool):
        return value
    elif isinstance(value, (int, float)):
        return value != 0
    elif isinstance(value, str):
        return len(value) > 0
    elif value is None:
        return False
    else:
        return True


class Animal:
    def __init__(self, name):
        self.name = name

    def speak(self):
        return (str(self.name) + str(" makes sound"))

    def getName(self):
        return self.name


class Dog(Animal):
    def __init__(self, name, breed):
        self.breed = breed
        self.name = name

    def speak(self):
        return (str(self.name) + str(" says woof!"))

    def getBreed(self):
        return self.breed

    @staticmethod
    def getSpecies():
        return "Canis lupus"

    @staticmethod
    def compareBreeds(breed1, breed2):
        return (breed1 == breed2)


dog = Dog("Buddy", "Golden Retriever")
print(dog.speak())
print(dog.getName())
print(dog.getBreed())
print(Dog.getSpecies())
print(Dog.compareBreeds("Labrador", "Labrador"))
print(Dog.compareBreeds("Poodle", "Dachshund"))
class Rectangle:
    def __init__(self, width, height):
        self.width = width
        self.height = height

    @property
    def area(self):
        return (self.width * self.height)

    @property
    def perimeter(self):
        return (2 * (self.width + self.height))


rect = Rectangle(5, 10)
print(rect.area)
print(rect.perimeter)
class Vehicle:
    def __init__(self, type):
        self.type = type

    def describe(self):
        return (str("Vehicle type: ") + str(self.type))

    @staticmethod
    def getMaxSpeed():
        return 200


class Car(Vehicle):
    def __init__(self, type, doors):
        self.type = type
        self.doors = doors

    def describe(self):
        return ((str((str("Car with ") + str(self.doors))) + str(" doors, type: ")) + self.type)

    @staticmethod
    def getMaxSpeed():
        return 300


car = Car("Sedan", 4)
print(car.describe())
print(Car.getMaxSpeed())